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
    uses_beside(name, main, LIB)
}

/// A second library: a class with a JSON codec and a private property, an
/// attribute, and a method whose parameter default reads a constant.
const CODEC_LIB: &str = "<?nvs\nnamespace Framework;\n\
    attribute Column on property { column: string }\n\
    #[Core\\Json\\Derive]\n\
    class Note {\n  public string $title = \"\";\n  private int $rank = 0;\n}\n\
    class Limits {\n  public const int LOW = 3;\n  \
    public static function clamp(int $n = self::LOW): int { return $n; }\n}\n";

/// [`uses_of`], beside the library `lib`.
fn uses_beside(name: &str, main: &str, lib: &str) -> (Vec<String>, Vec<String>) {
    let mut program = Program::new(name, &[("main.nvs", main), ("lib.nvs", lib)]);
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
fn an_unknown_member_on_an_unknown_type_is_a_miss_on_no_class() {
    let (_, missed) = uses_of(
        "uses-unknown-receiver",
        &main_with("var $m = new Framework\\Missing();\necho $m->title();"),
    );
    assert!(missed.contains(&"::title".to_owned()), "{missed:?}");
}

#[test]
fn a_subclass_file_uses_its_parent_and_misses_an_unknown_one() {
    let (used, missed) = uses_of(
        "uses-subclass",
        &main_with("class Mine extends Child {}\nclass Other extends Gone {}"),
    );
    assert!(used.contains(&"Framework\\Child".to_owned()), "{used:?}");
    assert!(missed.contains(&"Gone".to_owned()), "{missed:?}");
}

#[test]
fn a_doc_tag_uses_what_it_names_and_misses_an_unknown_member() {
    let (used, missed) = uses_of(
        "uses-see",
        &main_with(
            "class Page {\n  /// Shows the page.\n  /// @see Base::greet\n  /// @see Base::gone\n  \
             public static function show(): void {}\n}",
        ),
    );
    assert!(
        used.contains(&"Framework\\Base::greet".to_owned()),
        "{used:?}"
    );
    assert!(
        missed.contains(&"Framework\\Base::gone".to_owned()),
        "{missed:?}"
    );
}

#[test]
fn a_decode_uses_every_property_of_the_class_private_ones_included() {
    let (used, _) = uses_beside(
        "uses-decode",
        "<?nvs\nrequire 'lib.nvs';\nuse Framework\\Note;\n\
         var $note = Core\\Json::decodeAs<Note>(\"{}\");\n",
        CODEC_LIB,
    );
    for name in [
        "Framework\\Note",
        "Framework\\Note::$title",
        "Framework\\Note::$rank",
    ] {
        assert!(used.contains(&name.to_owned()), "{name}: {used:?}");
    }
}

#[test]
fn an_attribute_is_a_use_and_an_unknown_one_is_a_miss() {
    let (used, missed) = uses_beside(
        "uses-attribute",
        "<?nvs\nrequire 'lib.nvs';\nuse Framework\\Column;\n\
         class Row {\n  #[Column(column: \"id\")]\n  public string $id = \"\";\n  \
         #[Gone]\n  public string $name = \"\";\n}\n",
        CODEC_LIB,
    );
    assert!(used.contains(&"Framework\\Column".to_owned()), "{used:?}");
    assert!(missed.contains(&"Gone".to_owned()), "{missed:?}");
    assert!(missed.contains(&"Row::Gone".to_owned()), "{missed:?}");
}

#[test]
fn a_call_uses_the_constants_its_parameter_defaults_read() {
    let (used, _) = uses_beside(
        "uses-default",
        "<?nvs\nrequire 'lib.nvs';\nuse Framework\\Limits;\necho Limits::clamp();\n",
        CODEC_LIB,
    );
    assert!(
        used.contains(&"Framework\\Limits::clamp".to_owned()),
        "{used:?}"
    );
    assert!(
        used.contains(&"Framework\\Limits::LOW".to_owned()),
        "{used:?}"
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
