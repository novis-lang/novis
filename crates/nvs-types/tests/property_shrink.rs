//! `rule:testing/property-testing`'s shrinker: a failing input becomes the simplest input that
//! still fails, and every step stays inside the parameter's declared type and `gen:` range.

mod common;

use common::check_src_table;
use nvs_types::property::shrink::{MAX_SHRINK_CALLS, candidates, pieces, shrink};
use nvs_types::property::{ClassGen, Gen, Generated, Len, PropertyCase, ShapeGen, generate};

/// The property `members` declares in `class P`, after `prelude`'s declarations.
fn property(prelude: &str, members: &str) -> PropertyCase {
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\nuse Core\\Test\\Property;\n{prelude}\nclass P {{\n{members}\n}}\n"
    ));
    assert!(!diags.has_errors(), "fixture was refused: {diags:?}");
    exprs.properties("P").expect("a property row")[0].clone()
}

/// The first call's inputs of a run under `seed` for which `pick` returns `true`.
fn first_inputs(
    case: &PropertyCase,
    seed: u64,
    pick: impl Fn(&[Generated]) -> bool,
) -> Vec<Generated> {
    (0..case.runs)
        .map(|index| generate(case, seed, index).expect("inputs fit"))
        .find(|inputs| pick(inputs))
        .expect("a run has such an input")
}

/// Whether `generator` allows `value`, checked here apart from the shrinker's own check.
fn allowed(value: &Generated, generator: &Gen, classes: &[ClassGen]) -> bool {
    let length = |len: &Len, count: usize| {
        u32::try_from(count).is_ok_and(|count| len.min <= count && count <= len.max)
    };
    match (generator, value) {
        (Gen::Nullable(_), Generated::Null) | (Gen::Bool, Generated::Bool(_)) => true,
        (Gen::Nullable(inner), value) => allowed(value, inner, classes),
        (Gen::Int { min, max }, Generated::Int(n)) => min <= n && n <= max,
        (Gen::Uint { min, max }, Generated::Uint(n)) => min <= n && n <= max,
        (Gen::Float { min, max }, Generated::Float(n)) => min <= n && n <= max,
        (Gen::Decimal { min, max }, Generated::Decimal(text)) => {
            let bound = |end: &i64| end.to_string().parse::<f64>().expect("a number");
            text.parse::<f64>()
                .is_ok_and(|n| bound(min) <= n && n <= bound(max))
        }
        (Gen::Str { len, .. }, Generated::Str(text)) => length(len, pieces(text).len()),
        (Gen::Bytes { len, .. }, Generated::Bytes(raw)) => length(len, raw.len()),
        (Gen::Enum { values, .. }, Generated::Enum { value, .. }) => values.contains(value),
        (Gen::List { element, len }, Generated::List(items)) => {
            length(len, items.len()) && items.iter().all(|item| allowed(item, element, classes))
        }
        (Gen::Shape(fields), Generated::Shape(values)) => values.iter().all(|(name, value)| {
            fields
                .iter()
                .find(|field| field.name == *name)
                .is_some_and(|field| allowed(value, &field.generator, classes))
        }),
        (Gen::Class(label), Generated::Object { class, args }) => {
            let row = classes
                .iter()
                .find(|row| row.label == *label)
                .expect("a class row");
            class == label
                && args.len() == row.args.len()
                && args
                    .iter()
                    .zip(&row.args)
                    .all(|(arg, generator)| allowed(arg, generator, classes))
        }
        _ => false,
    }
}

#[test]
fn a_list_that_fails_from_three_elements_shrinks_to_three_zeros() {
    let case = property(
        "",
        "  #[Property]\n  public function p(array<int> $xs): void {}\n",
    );
    let inputs = first_inputs(
        &case,
        1,
        |inputs| matches!(&inputs[0], Generated::List(xs) if xs.len() >= 10 && xs.iter().any(|x| *x != Generated::Int(0))),
    );
    let (shrunk, calls) = shrink(
        inputs,
        &case.params,
        &case.classes,
        |args| matches!(&args[0], Generated::List(xs) if xs.len() >= 3),
        MAX_SHRINK_CALLS,
    );
    assert_eq!(shrunk, [Generated::List(vec![Generated::Int(0); 3])]);
    assert!(calls < MAX_SHRINK_CALLS, "{calls} calls");
}

#[test]
fn an_int_that_fails_from_ten_shrinks_to_ten() {
    let params = [Gen::Int {
        min: i64::MIN,
        max: i64::MAX,
    }];
    let fails = |args: &[Generated]| matches!(args[0], Generated::Int(n) if n >= 10);
    let (shrunk, _) = shrink(
        vec![Generated::Int(987_654_321)],
        &params,
        &[],
        fails,
        MAX_SHRINK_CALLS,
    );
    assert_eq!(shrunk, [Generated::Int(10)]);
}

#[test]
fn an_int_in_a_gen_range_shrinks_to_the_end_nearest_zero() {
    let case = property(
        "",
        "  #[Property(gen: {n: \"5..20\"})]\n  public function p(int $n): void {}\n",
    );
    let inputs = first_inputs(&case, 3, |inputs| inputs[0] != Generated::Int(5));
    let (shrunk, calls) = shrink(
        inputs,
        &case.params,
        &case.classes,
        |_| true,
        MAX_SHRINK_CALLS,
    );
    assert_eq!(shrunk, [Generated::Int(5)]);
    assert_eq!(calls, 1, "the range's low end is the first candidate");
}

#[test]
fn a_nullable_int_shrinks_to_null() {
    let case = property("", "  #[Property]\n  public function p(?int $n): void {}\n");
    let inputs = first_inputs(&case, 5, |inputs| inputs[0] != Generated::Null);
    let (shrunk, calls) = shrink(
        inputs,
        &case.params,
        &case.classes,
        |_| true,
        MAX_SHRINK_CALLS,
    );
    assert_eq!(shrunk, [Generated::Null]);
    assert_eq!(calls, 1);
}

#[test]
fn a_string_that_fails_from_two_pieces_shrinks_to_two_pieces_of_a() {
    let case = property(
        "",
        "  #[Property]\n  public function p(string $s): void {}\n",
    );
    let inputs = first_inputs(
        &case,
        11,
        |inputs| matches!(&inputs[0], Generated::Str(text) if pieces(text).len() >= 4 && !text.is_ascii()),
    );
    let (shrunk, _) = shrink(
        inputs,
        &case.params,
        &case.classes,
        |args| matches!(&args[0], Generated::Str(text) if pieces(text).len() >= 2),
        MAX_SHRINK_CALLS,
    );
    assert_eq!(shrunk, [Generated::Str("aa".to_owned())]);
}

#[test]
fn a_string_is_cut_between_whole_pieces() {
    let text = "e\u{301}x\u{1F1E9}\u{1F1EA}";
    assert_eq!(pieces(text), ["e\u{301}", "x", "\u{1F1E9}\u{1F1EA}"]);
    let generator = Gen::Str {
        len: Len { min: 1, max: 10 },
        tainted: false,
    };
    for candidate in candidates(&Generated::Str(text.to_owned()), &generator, &[]) {
        let Generated::Str(shorter) = &candidate else {
            panic!("not a string: {candidate:?}");
        };
        assert!(
            pieces(shorter)
                .iter()
                .all(|piece| pieces(text).contains(piece) || *piece == "a"),
            "{shorter:?} splits a piece"
        );
        assert!(!shorter.is_empty(), "the shortest length is 1");
    }
}

#[test]
fn the_budget_stops_the_loop() {
    let params = [Gen::Int {
        min: i64::MIN,
        max: i64::MAX,
    }];
    let fails = |args: &[Generated]| matches!(args[0], Generated::Int(n) if n >= 10);
    // `0` passes, `500000` fails and is kept, and `0` passes again: three calls.
    let (shrunk, calls) = shrink(vec![Generated::Int(1_000_000)], &params, &[], fails, 3);
    assert_eq!(calls, 3);
    assert_eq!(shrunk, [Generated::Int(500_000)]);

    let (shrunk, calls) = shrink(vec![Generated::Int(1_000_000)], &params, &[], fails, 0);
    assert_eq!(calls, 0);
    assert_eq!(shrunk, [Generated::Int(1_000_000)]);
}

#[test]
fn a_shape_drops_its_optional_field_first() {
    let shape = Gen::Shape(vec![
        ShapeGen {
            name: "id".to_owned(),
            generator: Gen::Int { min: 1, max: 9 },
            required: true,
        },
        ShapeGen {
            name: "tag".to_owned(),
            generator: Gen::Str {
                len: Len { min: 1, max: 5 },
                tainted: false,
            },
            required: false,
        },
    ]);
    let value = Generated::Shape(vec![
        ("id".to_owned(), Generated::Int(7)),
        ("tag".to_owned(), Generated::Str("xyz".to_owned())),
    ]);
    let list = candidates(&value, &shape, &[]);
    assert_eq!(
        list[0],
        Generated::Shape(vec![("id".to_owned(), Generated::Int(7))])
    );
    assert!(list.iter().all(|candidate| allowed(candidate, &shape, &[])));

    let (shrunk, _) = shrink(vec![value], &[shape], &[], |_| true, MAX_SHRINK_CALLS);
    assert_eq!(
        shrunk,
        [Generated::Shape(vec![("id".to_owned(), Generated::Int(1))])]
    );
}

#[test]
fn every_candidate_stays_in_its_range() {
    let case = property(
        "enum Color { Red, Green, Blue }\nclass Point {\n  \
         public function constructor(public int $x, public ?Point $next) {}\n}",
        "  #[Property(gen: {a: \"-50..-5\", b: \"7..1000\", f: \"1.5..2.5\", d: \"-3..3\", \
         s: \"2..6\", raw: \"1..3\", xs: \"1..4\", n: \"3..9\"})]\n  \
         public function p(int $a, uint $b, float $f, decimal $d, string $s, bytes $raw, \
         array<int> $xs, ?int $n, bool $flag, Color $c, Point $point): void {}\n",
    );
    let mut checked = 0;
    for seed in 1..=4 {
        for index in (0..case.runs).step_by(7) {
            let inputs = generate(&case, seed, index).expect("inputs fit");
            for (value, generator) in inputs.iter().zip(&case.params) {
                assert!(
                    allowed(value, generator, &case.classes),
                    "generated {value:?}"
                );
                for candidate in candidates(value, generator, &case.classes) {
                    assert!(
                        allowed(&candidate, generator, &case.classes),
                        "{candidate:?} from {value:?} is outside {generator:?}"
                    );
                    assert_ne!(&candidate, value);
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 1000, "only {checked} candidates");
}
