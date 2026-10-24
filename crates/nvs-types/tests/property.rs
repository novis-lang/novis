//! `rule:testing/property-testing`'s `#[Property]`: the generator each parameter gets from its
//! declared type, the `gen:` ranges that narrow one, and the inputs a seed generates.
//!
//! The runner that calls a property is not part of this crate, so what is asserted here is the
//! table the compile builds and the values [`nvs_types::property::generate`] makes from it.

mod common;

use common::{check_src_table, check_src_table_allowing_errors};
use nvs_diagnostics::code;
use nvs_types::property::{
    ClassGen, Gen, Generated, Len, PropertyCase, TooLarge, generate, generate::CLUSTERS,
};

/// The property `members` declares in `class P`, after `prelude`'s declarations.
fn property(prelude: &str, members: &str) -> PropertyCase {
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\nuse Core\\Test\\Property;\n{prelude}\nclass P {{\n{members}\n}}\n"
    ));
    assert!(!diags.has_errors(), "fixture was refused: {diags:?}");
    exprs.properties("P").expect("a property row")[0].clone()
}

/// The diagnostic codes `members` draws.
fn refused(prelude: &str, members: &str) -> Vec<nvs_diagnostics::Code> {
    let (diags, _) = check_src_table_allowing_errors(&format!(
        "<?nvs\nuse Core\\Test\\Property;\nuse Core\\Test;\n{prelude}\nclass P {{\n{members}\n}}\n"
    ));
    diags.iter().filter_map(|d| d.code).collect()
}

/// Every call's inputs of a whole run, under `seed`.
fn run(case: &PropertyCase, seed: u64) -> Vec<Vec<Generated>> {
    (0..case.runs)
        .map(|index| generate(case, seed, index).expect("inputs fit"))
        .collect()
}

#[test]
fn each_scalar_parameter_gets_the_generator_of_its_type() {
    let case = property(
        "enum Color { Red, Green, Blue }",
        "  #[Property]\n  public function p(bool $b, int $i, uint $u, float $f, decimal $d, \
         string $s, bytes $raw, ?int $n, Color $c, array<int> $xs): void {}\n",
    );
    assert_eq!(case.runs, 100);
    let full = Len { min: 0, max: 100 };
    assert_eq!(case.params[0], Gen::Bool);
    assert_eq!(
        case.params[1],
        Gen::Int {
            min: i64::MIN,
            max: i64::MAX
        }
    );
    assert_eq!(
        case.params[2],
        Gen::Uint {
            min: 0,
            max: u64::MAX
        }
    );
    assert!(matches!(case.params[3], Gen::Float { .. }));
    assert!(matches!(case.params[4], Gen::Decimal { .. }));
    assert_eq!(
        case.params[5],
        Gen::Str {
            len: full,
            tainted: false
        }
    );
    assert_eq!(
        case.params[6],
        Gen::Bytes {
            len: full,
            tainted: false
        }
    );
    assert!(matches!(&case.params[7], Gen::Nullable(inner) if matches!(**inner, Gen::Int { .. })));
    assert!(matches!(&case.params[8], Gen::Enum(values) if values.len() == 3));
    assert!(
        matches!(&case.params[9], Gen::List { element, len } if **element == Gen::Int { min: i64::MIN, max: i64::MAX } && *len == full)
    );

    // Every call's values have the declared types, and `?int` is sometimes `null`.
    let mut saw_null = false;
    for inputs in run(&case, 7) {
        assert!(matches!(inputs[0], Generated::Bool(_)));
        assert!(matches!(inputs[1], Generated::Int(_)));
        assert!(matches!(inputs[2], Generated::Uint(_)));
        assert!(matches!(inputs[3], Generated::Float(f) if f.is_finite()));
        assert!(
            matches!(&inputs[4], Generated::Decimal(text) if text.parse::<f64>().is_ok()),
            "{:?}",
            inputs[4]
        );
        assert!(matches!(inputs[5], Generated::Str(_)));
        assert!(matches!(&inputs[6], Generated::Bytes(raw) if raw.len() <= 100));
        saw_null |= inputs[7] == Generated::Null;
        assert!(matches!(&inputs[8], Generated::Enum(v) if (0..3).contains(v)));
        assert!(matches!(&inputs[9], Generated::List(xs) if xs.len() <= 100));
    }
    assert!(saw_null);
}

#[test]
fn a_user_class_is_built_from_its_constructor_parameters() {
    let case = property(
        "class Point {\n  public function constructor(public int $x, public ?Point $next) {}\n}",
        "  #[Property]\n  public function p(Point $point): void {}\n",
    );
    assert_eq!(case.params, [Gen::Class("Point".to_owned())]);
    // The class reaches itself through a `?Point`, so the table is finite.
    assert_eq!(
        case.classes,
        [ClassGen {
            label: "Point".to_owned(),
            args: vec![
                Gen::Int {
                    min: i64::MIN,
                    max: i64::MAX
                },
                Gen::Nullable(Box::new(Gen::Class("Point".to_owned()))),
            ],
        }]
    );
    for inputs in run(&case, 3) {
        let Generated::Object { class, args } = &inputs[0] else {
            panic!("not an object: {inputs:?}");
        };
        assert_eq!(class, "Point");
        assert_eq!(args.len(), 2);
    }
}

#[test]
fn a_class_that_contains_itself_with_no_null_between_has_no_generator() {
    let codes = refused(
        "class Loop {\n  public function constructor(public Loop $next) {}\n}",
        "  #[Property]\n  public function p(Loop $l): void {}\n",
    );
    assert!(codes.contains(&code::E_PROPERTY_PARAMETER), "{codes:?}");
}

#[test]
fn a_generated_string_includes_grapheme_clusters_of_several_code_points() {
    let case = property(
        "",
        "  #[Property]\n  public function p(string $s): void {}\n",
    );
    let strings: Vec<String> = run(&case, 11)
        .into_iter()
        .map(|inputs| match &inputs[0] {
            Generated::Str(text) => text.clone(),
            other => panic!("not a string: {other:?}"),
        })
        .collect();
    assert!(
        strings
            .iter()
            .any(|text| CLUSTERS.iter().any(|cluster| text.contains(cluster))),
        "no multi-code-point cluster in a whole run"
    );
    assert!(strings.iter().any(|text| text.contains('\u{301}')));
    assert!(strings.iter().any(|text| !text.is_ascii()));
    assert!(
        strings.iter().any(String::is_empty),
        "the first call is size 0"
    );
}

#[test]
fn the_same_seed_gives_the_same_inputs() {
    let case = property(
        "",
        "  #[Property(runs: 50)]\n  public function p(int $i, string $s, array<float> $xs): void {}\n",
    );
    assert_eq!(case.runs, 50);
    assert_eq!(run(&case, 42), run(&case, 42));
    assert_ne!(run(&case, 42), run(&case, 43));
}

#[test]
fn a_gen_range_narrows_a_number_and_a_length() {
    let case = property(
        "",
        "  #[Property(gen: {age: \"0..120\", name: \"1..3\", n: \"5..6\"})]\n  \
         public function p(int $age, string $name, ?int $n): void {}\n",
    );
    assert_eq!(case.params[0], Gen::Int { min: 0, max: 120 });
    for inputs in run(&case, 5) {
        assert!(matches!(inputs[0], Generated::Int(age) if (0..=120).contains(&age)));
        assert!(matches!(&inputs[1], Generated::Str(name) if !name.is_empty()));
        assert!(matches!(inputs[2], Generated::Null | Generated::Int(5 | 6)));
    }
}

#[test]
fn a_gen_entry_that_does_not_fit_the_method_is_refused() {
    for members in [
        "  #[Property(gen: {other: \"0..1\"})]\n  public function p(int $i): void {}\n",
        "  #[Property(gen: {b: \"0..1\"})]\n  public function p(bool $b): void {}\n",
        "  #[Property(gen: {i: \"9..1\"})]\n  public function p(int $i): void {}\n",
        "  #[Property(gen: {s: \"0..99999\"})]\n  public function p(string $s): void {}\n",
        "  #[Property(gen: {i: \"many\"})]\n  public function p(int $i): void {}\n",
    ] {
        let codes = refused("", members);
        assert!(
            codes.contains(&code::E_PROPERTY_PARAMETER),
            "{members}: {codes:?}"
        );
    }
}

#[test]
fn a_parameter_type_with_no_values_is_refused() {
    let codes = refused(
        "interface Shape {}",
        "  #[Property]\n  public function p(Shape $s): void {}\n",
    );
    assert!(codes.contains(&code::E_PROPERTY_PARAMETER), "{codes:?}");
}

#[test]
fn a_property_has_the_shape_of_a_test_and_is_test_code() {
    let codes = refused(
        "",
        "  #[Property]\n  public static function p(int $i): void {}\n",
    );
    assert!(codes.contains(&code::E_TEST_METHOD_SHAPE), "{codes:?}");
    let codes = refused(
        "",
        "  #[Property]\n  #[Test]\n  public function p(int $i): void {}\n",
    );
    assert!(codes.contains(&code::E_TEST_METHOD_SHAPE), "{codes:?}");
    let codes = refused(
        "",
        "  #[Property(retries: 2, because: \"x\")]\n  public function p(int $i): void {}\n",
    );
    assert!(codes.contains(&code::E_UNKNOWN_OPTION), "{codes:?}");
    let codes = refused(
        "",
        "  #[Property(runs: 0)]\n  public function p(int $i): void {}\n",
    );
    assert!(codes.contains(&code::E_TEST_OPTION_VALUE), "{codes:?}");

    let (diags, exprs) = check_src_table(
        "<?nvs\nuse Core\\Test\\Property;\nclass P {\n  #[Property]\n  \
         public function p(int $i): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(exprs.is_test_code("P::p"));
    assert!(
        exprs.tests("P").is_none(),
        "a property is not in the `#[Test]` table"
    );
}

#[test]
fn an_input_whose_shortest_form_is_too_large_is_refused_before_it_is_built() {
    let long = Len {
        min: 10_000,
        max: 10_000,
    };
    // `gen:` bounds the outer length alone, so the nested one is set on the row directly.
    let mut case = property(
        "",
        "  #[Property]\n  public function p(array<string> $xs): void {}\n",
    );
    case.params = vec![Gen::List {
        element: Box::new(Gen::Str {
            len: long,
            tainted: false,
        }),
        len: long,
    }];
    assert_eq!(generate(&case, 1, 0), Err(TooLarge));
}
