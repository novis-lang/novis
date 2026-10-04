//! The natural-order permutation of a sort's entries, computed by **key**
//! rather than by comparison — the path [`crate::arr`]'s `sort` and `sortByKey`
//! take when no comparator was given and every compared value sits in one row
//! of [`crate::ordering::compare_values`]'s table.
//!
//! # Why two sorts
//!
//! A comparison in `Core\Arr::sort` can **fail**: a comparator callback can
//! throw, and two values of incomparable types are a throw of the member's
//! own. That is why the member carries a hand-written merge sort with a
//! fallible comparison, and why no library sort could replace it outright —
//! every one of them takes an infallible comparator.
//!
//! But the failure is decidable *before* the sort starts. Without a
//! comparator, a subject whose values are all `int`/`uint`, all `float`, all
//! `string`, all `bool` or all `null` can never throw, because every pair has
//! a row. One pass over the tags settles it, and such a subject is handed to
//! [`brainsort`] as a key sort: each value maps to an integer or a byte string
//! whose order *is* [`compare_values`](crate::ordering::compare_values)'s, and
//! the crate sorts by radix where the key allows it. Anything else — a
//! `decimal`, a `bytes`, an `int` beside a `float`, an array, an object, a
//! comparator — stays on the merge sort, which throws exactly as before. The
//! two paths answer the same permutation on every input they share: a stable
//! sort's output is unique given a total order, and the tests below hold the
//! key mapping to the ordering, index for index.
//!
//! # The key mapping
//!
//! | row | key | why it orders as `compare_values` does |
//! |---|---|---|
//! | all `int` | `i64` | exact |
//! | all `uint` | `u64` | exact |
//! | `int` and `uint` mixed | `i128` | the ordering widens both to `i128` |
//! | all `float` | `i64` | the bits `f64::total_cmp` compares, after [`ordered`](crate::ordering::ordered) has read every `NaN` as the one below every number; brainsort's own `f64` key (`-0.0 == 0.0`, a `NaN` by its sign) is deliberately not used |
//! | all `string` | `&[u8]` | bytewise, shorter prefix first |
//! | all `bool` | `bool` | `false` before `true` |
//! | all `null` | none | every pair is equal, so the permutation is already the answer |
//!
//! `Core\Order::Desc` wraps the key in [`brainsort::Desc`], which reverses the
//! comparison and not the result — the same rule the merge sort applies, so
//! equal keys keep their input order under either order.
//!
//! # What it spends
//!
//! Per `rule:programs/memory-priority`'s *Say what you spend*, transient and
//! per call: the crate builds one record per element — 16 bytes for a 64-bit
//! key, 24 for the `i128` row, 16 for a string — plus scratch of about half
//! the records, count tables of at most 64 KiB, and a buffer of `n` `usize`s
//! through which the permutation is applied; about 40 bytes an element,
//! against the merge sort's 16. Every block is freed on return. Below 33
//! elements nothing is allocated at all — the crate insertion-sorts in place.
//!
//! **brainsort's block cache is off.** The crate keeps freed blocks of 64 KiB
//! and more for the next sort of the process, up to 32 MiB, shared by every
//! thread under one mutex. In Novis that is memory attributable to no request,
//! which the rule above calls a hole rather than a trade-off, so [`cache_off`]
//! sets the limit to zero before the first sort. The allocate and free paths
//! still take the (uncontended) mutex once per large block before consulting
//! the limit; the cost is measured in `docs/perf/brainsort-evaluation.md` and
//! is paid.
//!
//! # What cannot go wrong, and what is contained
//!
//! The Rust key closures cannot fail: every value was classified before the sort
//! began, so each `as_*` read answers. Allocation failure inside the crate
//! falls back to the standard library's stable sort with the same order. A
//! panic inside the crate is contained to one request by the helper
//! boundary's `catch_unwind`, like any other. The crate's limits — 2^32 − 1
//! elements, strings under 2^32 bytes — send a call to the standard library's
//! stable sort rather than refusing it.

use nvs_runtime::{NvsStr, Tag, Value};

use crate::ordering::ordered;

/// Sorts `permutation` by the natural order of the values it indexes, or
/// answers `false` and leaves it untouched when the values do not all share
/// one row — in which case the caller's merge sort is the path, and the throw
/// it raises is the answer.
///
/// `permutation` is the identity on entry, as [`crate::arr`] builds it; the
/// crate's prescan is what recognises already-sorted input, so nothing here
/// looks before it sorts.
pub(crate) fn natural(permutation: &mut [usize], values: &[Value], descending: bool) -> bool {
    if permutation.len() < 2 {
        return true;
    }
    let Some(row) = row_of(values) else {
        return false;
    };
    cache_off();
    match row {
        Row::Int => by_key(permutation, descending, |i| {
            values[i].as_int().unwrap_or_default()
        }),
        Row::Uint => by_key(permutation, descending, |i| {
            values[i].as_uint().unwrap_or_default()
        }),
        Row::IntAndUint => by_key(permutation, descending, |i| {
            let value = values[i];
            value
                .as_int()
                .map(i128::from)
                .or_else(|| value.as_uint().map(i128::from))
                .unwrap_or_default()
        }),
        Row::Float => by_key(permutation, descending, |i| {
            float_key(values[i].as_float().unwrap_or_default())
        }),
        Row::Str => by_key(permutation, descending, |i| {
            values[i].as_str_bytes().unwrap_or_default()
        }),
        Row::Bool => by_key(permutation, descending, |i| {
            values[i].as_bool().unwrap_or_default()
        }),
        Row::Null => {}
    }
    true
}

/// Sorts `permutation` by the bytes of the keys it indexes — `sortByKey`'s
/// natural order, which is `[u8]`'s and never fails.
pub(crate) fn by_bytes(permutation: &mut [usize], keys: &[NvsStr], descending: bool) {
    if permutation.len() < 2 {
        return;
    }
    cache_off();
    by_key(permutation, descending, |i| keys[i].as_bytes());
}

/// The one row every value of a subject sits in, per
/// [`compare_values`](crate::ordering::compare_values)'s table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Row {
    Int,
    Uint,
    /// Both integer tags, which the ordering compares exactly through `i128`.
    IntAndUint,
    Float,
    Str,
    Bool,
    Null,
}

/// The row the values share, or `None` for a subject that has no single row
/// — two different rows, or a value whose tag has no natural order at all.
fn row_of(values: &[Value]) -> Option<Row> {
    let mut row: Option<Row> = None;
    for value in values {
        let this = match value.tag()? {
            Tag::Int => Row::Int,
            Tag::Uint => Row::Uint,
            Tag::Float => Row::Float,
            Tag::Str => Row::Str,
            Tag::Bool => Row::Bool,
            Tag::Null => Row::Null,
            // `decimal` and `bytes` included: the merge sort is the path that
            // knows what `compare_values` answers for them, and for a mixed
            // numeric subject.
            _ => return None,
        };
        row = Some(match row {
            None => this,
            Some(so_far) if so_far == this => so_far,
            Some(Row::Int | Row::Uint | Row::IntAndUint)
                if matches!(this, Row::Int | Row::Uint) =>
            {
                Row::IntAndUint
            }
            Some(_) => return None,
        });
    }
    row
}

/// Sorts `permutation` by `key`, stably, in the order asked for.
fn by_key<K: brainsort::Key>(
    permutation: &mut [usize],
    descending: bool,
    mut key: impl FnMut(usize) -> K,
) {
    if descending {
        brainsort::sort_by_key(permutation, |&i| brainsort::Desc(key(i)));
    } else {
        brainsort::sort_by_key(permutation, |&i| key(i));
    }
}

/// The `i64` whose order under `<` is `f64::total_cmp`'s over the value
/// [`ordered`] reads: the sign bit flipped into the top bit of a two's
/// complement number, and every other bit inverted for a negative value, which
/// is the transformation `total_cmp` itself applies before comparing.
fn float_key(value: f64) -> i64 {
    let bits = ordered(value).to_bits();
    let mask = if bits >> 63 == 1 { u64::MAX >> 1 } else { 0 };
    i64::from_ne_bytes((bits ^ mask).to_ne_bytes())
}

/// Turns brainsort's process-wide block cache off, once — the module doc's
/// *What it spends* owns why.
fn cache_off() {
    static OFF: std::sync::Once = std::sync::Once::new();
    OFF.call_once(|| brainsort::set_memory_cache_limit(0));
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use nvs_runtime::{Decimal, NvsArray, NvsStr, Value};

    use super::{Row, by_bytes, float_key, natural, row_of};
    use crate::arr::merge_sort;
    use crate::ordering::{compare_values, ordered};

    /// Releases a reference this test owns.
    #[expect(
        unsafe_code,
        reason = "each caller built the one reference it drops here"
    )]
    fn dropped(value: Value) {
        unsafe {
            value.release();
        }
    }

    fn text(s: &str) -> Value {
        Value::str(NvsStr::new(s.as_bytes()))
    }

    /// A small deterministic generator, so a failing case can be named by its
    /// seed rather than reproduced by luck.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 11
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n.max(1)
        }
        /// A position below `n`, as a `usize` — the index a test draws.
        fn index(&mut self, n: usize) -> usize {
            let bound = u64::try_from(n.max(1)).expect("a length fits a u64");
            usize::try_from(self.next() % bound).expect("below a usize bound")
        }
    }

    /// The permutation the merge sort answers over `values`, through the same
    /// comparison the member uses.
    fn reference(values: &[Value], descending: bool) -> Vec<usize> {
        let mut permutation: Vec<usize> = (0..values.len()).collect();
        // Every value here is a scalar, so the context the object row needs is
        // one no comparison below reaches for.
        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut compare = |left: usize, right: usize| {
            let ordering = compare_values(&mut ctx, &values[left], &values[right], "test")?;
            Ok(if descending {
                ordering.reverse()
            } else {
                ordering
            })
        };
        merge_sort(&mut permutation, &mut compare).expect("one row never throws");
        permutation
    }

    /// Asserts the fast path answers the merge sort's permutation, index for
    /// index — which proves stability as well as order, since a stable sort's
    /// permutation is unique.
    fn agrees(values: &[Value], label: &str) {
        for descending in [false, true] {
            let mut permutation: Vec<usize> = (0..values.len()).collect();
            assert!(
                natural(&mut permutation, values, descending),
                "{label}: a single-row subject must take the fast path"
            );
            let expected = reference(values, descending);
            assert_eq!(
                permutation, expected,
                "{label}, descending={descending}: the permutations differ"
            );
        }
    }

    /// The shapes an input comes in, each of which sends brainsort down a
    /// different route — sorted and reversed input are recognised by the
    /// prescan, nearly sorted takes the displaced-element route, and the rest
    /// go through records.
    const SHAPES: &[&str] = &[
        "random",
        "sorted",
        "reversed",
        "nearly-sorted",
        "few-distinct",
        "all-equal",
        "runs",
        "organ-pipe",
        "two-values",
    ];

    /// Sizes straddling every threshold the crate has: the small sort at 32,
    /// the split at about a thousand, the key-inference floor at 4096.
    const SIZES: &[usize] = &[
        0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 33, 63, 64, 65, 100, 255, 256, 257, 999, 1000, 1001,
        1023, 1024, 1025, 4095, 4096, 4097, 5000, 10_000, 20_000,
    ];

    /// `n` integers in `shape`, as a sequence of `u64` a row builder maps.
    fn shaped(rng: &mut Lcg, n: usize, shape: &str) -> Vec<u64> {
        let mut out: Vec<u64> = match shape {
            "random" => (0..n).map(|_| rng.next()).collect(),
            "sorted" => (0..n).map(|i| i as u64 * 3).collect(),
            "reversed" => (0..n).rev().map(|i| i as u64 * 3).collect(),
            "nearly-sorted" => (0..n).map(|i| i as u64).collect(),
            "few-distinct" => (0..n).map(|_| rng.below(5)).collect(),
            "all-equal" => vec![7; n],
            "runs" => (0..n)
                .map(|i| (i % 64) as u64 * 1000 + (i / 64) as u64)
                .collect(),
            "organ-pipe" => (0..n)
                .map(|i| if i < n / 2 { i as u64 } else { (n - i) as u64 })
                .collect(),
            "two-values" => (0..n).map(|_| rng.below(2) * 1_000_000).collect(),
            other => panic!("no shape {other}"),
        };
        if shape == "nearly-sorted" && n >= 2 {
            for _ in 0..(n / 16).max(1) {
                let a = rng.index(n);
                let b = rng.index(n);
                out.swap(a, b);
            }
        }
        out
    }

    fn ints(raw: &[u64]) -> Vec<Value> {
        raw.iter()
            .map(|&v| Value::int(i64::from_ne_bytes(v.to_ne_bytes()) >> 1))
            .collect()
    }

    fn uints(raw: &[u64]) -> Vec<Value> {
        raw.iter().map(|&v| Value::uint(v)).collect()
    }

    fn int_and_uint(raw: &[u64]) -> Vec<Value> {
        raw.iter()
            .enumerate()
            .map(|(i, &v)| {
                if i % 3 == 0 {
                    Value::uint(v)
                } else {
                    Value::int(i64::from_ne_bytes(v.to_ne_bytes()))
                }
            })
            .collect()
    }

    fn floats(raw: &[u64]) -> Vec<Value> {
        raw.iter()
            .enumerate()
            .map(|(i, &v)| {
                // Some of every kind, so the corner bits are in every shape.
                let f = match i % 11 {
                    0 => f64::NAN,
                    1 => -f64::NAN,
                    2 => 0.0,
                    3 => -0.0,
                    4 => f64::INFINITY,
                    5 => f64::NEG_INFINITY,
                    _ => (v as f64 / 1e12) - 4e6,
                };
                Value::float(f)
            })
            .collect()
    }

    fn strings(raw: &[u64]) -> Vec<Value> {
        raw.iter()
            .enumerate()
            .map(|(i, &v)| {
                let s = match i % 7 {
                    0 => String::new(),
                    1 => "a".repeat((v % 5) as usize),
                    2 => format!("{}", v % 1000),
                    3 => format!("{:0>300}{}", "", v % 3),
                    4 => String::from_utf8_lossy(&[0xF0, 0x9F, 0x98, 0x80]).into_owned(),
                    _ => format!("k{v}"),
                };
                text(&s)
            })
            .collect()
    }

    fn bools(raw: &[u64]) -> Vec<Value> {
        raw.iter().map(|&v| Value::bool(v % 2 == 1)).collect()
    }

    fn nulls(raw: &[u64]) -> Vec<Value> {
        raw.iter().map(|_| Value::null()).collect()
    }

    fn release_all(values: Vec<Value>) {
        for value in values {
            if value.as_str_bytes().is_some() {
                dropped(value);
            }
        }
    }

    /// Every row × every shape × every size × both orders: the fast path and
    /// the merge sort answer the same permutation.
    #[test]
    fn every_row_shape_and_size_answers_the_merge_sorts_permutation() {
        type Build = fn(&[u64]) -> Vec<Value>;
        let rows: &[(&str, Build)] = &[
            ("int", ints),
            ("uint", uints),
            ("int+uint", int_and_uint),
            ("float", floats),
            ("string", strings),
            ("bool", bools),
            ("null", nulls),
        ];
        let mut rng = Lcg(0x5EED);
        for (row, build) in rows {
            for shape in SHAPES {
                for &n in SIZES {
                    let raw = shaped(&mut rng, n, shape);
                    let values = build(&raw);
                    agrees(&values, &format!("{row}/{shape}/{n}"));
                    release_all(values);
                }
            }
        }
    }

    /// The float key is `total_cmp` over [`ordered`], bit for bit, on the
    /// corners and on random bit patterns — which is the whole claim the
    /// `float` row rests on.
    #[test]
    fn the_float_key_orders_exactly_as_total_cmp_over_ordered_does() {
        let corners = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::from_bits(0x8000_0000_0000_0001),
            f64::MAX,
            f64::MIN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            f64::from_bits(0x7FF0_0000_0000_0001),
            f64::from_bits(0xFFF0_0000_0000_0001),
            f64::from_bits(0x7FF8_0000_0000_0000),
            f64::from_bits(0xFFF8_0000_0000_0000),
            f64::EPSILON,
            1e308,
            -1e308,
            123_456.789,
        ];
        let mut rng = Lcg(42);
        let mut sample: Vec<f64> = corners.to_vec();
        sample.extend((0..2000).map(|_| f64::from_bits(rng.next() << 11 | rng.below(2048))));
        for &a in &sample {
            for &b in &sample {
                let expected = ordered(a).total_cmp(&ordered(b));
                let got = float_key(a).cmp(&float_key(b));
                assert_eq!(
                    got,
                    expected,
                    "{a:?} ({:#x}) against {b:?} ({:#x})",
                    a.to_bits(),
                    b.to_bits()
                );
            }
        }
        // And every NaN is one NaN: equal to each other, below everything.
        assert_eq!(float_key(f64::NAN), float_key(-f64::NAN));
        assert_eq!(
            float_key(f64::NAN),
            float_key(f64::from_bits(0x7FF0_0000_0000_0001))
        );
        assert_eq!(
            float_key(f64::NAN).cmp(&float_key(f64::NEG_INFINITY)),
            Ordering::Less
        );
        assert_eq!(float_key(-0.0).cmp(&float_key(0.0)), Ordering::Less);
    }

    /// The integer corners in every mix: the `i64` ends, the `u64` end, and
    /// the `int`/`uint` boundary where widening is what keeps the order exact.
    #[test]
    fn integer_corners_order_exactly_in_every_mix() {
        let corners: Vec<Value> = vec![
            Value::int(i64::MIN),
            Value::int(i64::MIN + 1),
            Value::int(-1),
            Value::int(0),
            Value::int(1),
            Value::int(i64::MAX - 1),
            Value::int(i64::MAX),
            Value::uint(0),
            Value::uint(1),
            Value::uint(i64::MAX as u64),
            Value::uint(i64::MAX as u64 + 1),
            Value::uint(u64::MAX - 1),
            Value::uint(u64::MAX),
        ];
        let mut rng = Lcg(7);
        for round in 0..500 {
            let n = 1 + rng.index(80);
            let values: Vec<Value> = (0..n).map(|_| corners[rng.index(corners.len())]).collect();
            agrees(&values, &format!("mix round {round}"));
        }
        // Each pure row too, so the `i64` and `u64` keys are held on their
        // own ends and not only through `i128`.
        let only_ints: Vec<Value> = corners
            .iter()
            .copied()
            .filter(|v| v.as_int().is_some())
            .collect();
        let only_uints: Vec<Value> = corners
            .iter()
            .copied()
            .filter(|v| v.as_uint().is_some())
            .collect();
        for round in 0..200 {
            let n = 33 + rng.index(200);
            let ints: Vec<Value> = (0..n)
                .map(|_| only_ints[rng.index(only_ints.len())])
                .collect();
            agrees(&ints, &format!("int ends round {round}"));
            let uints: Vec<Value> = (0..n)
                .map(|_| only_uints[rng.index(only_uints.len())])
                .collect();
            agrees(&uints, &format!("uint ends round {round}"));
        }
    }

    /// String corners: the empty string, prefixes of one another, a long
    /// shared prefix, high bytes, and the numeric strings the ordering
    /// deliberately compares bytewise.
    #[test]
    fn string_corners_order_bytewise_shorter_prefix_first() {
        let corner_texts = [
            "", "a", "aa", "aaa", "b", "ab", "10", "9", "100", "\u{7f}", "\u{80}", "\u{ff}", "é",
            "z", "Z", "0", " ", "a b", "a\u{0}b", "a\u{0}",
        ];
        let long_a = format!("{}a", "x".repeat(300));
        let long_b = format!("{}b", "x".repeat(300));
        let long_short = "x".repeat(300);
        let mut pool: Vec<Value> = corner_texts.iter().map(|s| text(s)).collect();
        pool.push(text(&long_a));
        pool.push(text(&long_b));
        pool.push(text(&long_short));
        let mut rng = Lcg(99);
        for round in 0..500 {
            let n = 1 + rng.index(120);
            let values: Vec<Value> = (0..n).map(|_| pool[rng.index(pool.len())]).collect();
            agrees(&values, &format!("string round {round}"));
        }
        release_all(pool);
    }

    /// A subject that is not one row is declined, with the permutation left
    /// as it was — so the merge sort, and its throw, is what the member
    /// answers, exactly as before.
    #[test]
    fn a_mixed_or_orderless_subject_is_declined_untouched() {
        let s = text("s");
        let b = Value::bytes(NvsStr::new(b"b"));
        let arr = Value::array(NvsArray::new());
        let dec = Value::decimal(Decimal::from_i64(1));
        let mixes: Vec<(&str, Vec<Value>)> = vec![
            (
                "int and float",
                vec![Value::int(1), Value::float(2.0), Value::int(3)],
            ),
            ("int and string", vec![Value::int(1), s, Value::int(3)]),
            ("int and decimal", vec![Value::int(1), dec, Value::int(3)]),
            ("float and decimal", vec![Value::float(1.0), dec]),
            ("bool and int", vec![Value::bool(true), Value::int(0)]),
            ("null and int", vec![Value::null(), Value::int(0)]),
            ("only bytes", vec![b, b]),
            ("string and bytes", vec![s, b]),
            ("an array", vec![arr, arr]),
            ("only decimals", vec![dec, dec]),
        ];
        for (label, values) in &mixes {
            for descending in [false, true] {
                let mut permutation: Vec<usize> = (0..values.len()).collect();
                assert!(
                    !natural(&mut permutation, values, descending),
                    "{label}: must decline"
                );
                let identity: Vec<usize> = (0..values.len()).collect();
                assert_eq!(permutation, identity, "{label}: declined means untouched");
            }
        }
        // And the same random layouts as the merge sort would throw on: one
        // string among ints, anywhere.
        let mut rng = Lcg(3);
        for _ in 0..400 {
            let n = 2 + rng.index(100);
            let at = rng.index(n);
            let values: Vec<Value> = (0..n)
                .map(|i| {
                    if i == at {
                        s
                    } else {
                        Value::int(i64::from_ne_bytes(rng.next().to_ne_bytes()))
                    }
                })
                .collect();
            assert_eq!(row_of(&values), None);
            let mut permutation: Vec<usize> = (0..n).collect();
            assert!(!natural(&mut permutation, &values, false));
            let mut ctx = nvs_runtime::Ctx::buffered();
            let mut compare =
                |l: usize, r: usize| compare_values(&mut ctx, &values[l], &values[r], "test");
            assert!(
                merge_sort(&mut permutation, &mut compare).is_err(),
                "the merge sort throws on it"
            );
        }
        dropped(s);
        dropped(b);
        dropped(arr);
    }

    /// The row classification itself, on the boundaries a sweep would not
    /// name: empty, one value, the integer mix in either order.
    #[test]
    fn the_row_is_classified_from_the_tags_alone() {
        assert_eq!(row_of(&[]), None);
        assert_eq!(row_of(&[Value::int(1)]), Some(Row::Int));
        assert_eq!(row_of(&[Value::uint(1)]), Some(Row::Uint));
        assert_eq!(
            row_of(&[Value::int(1), Value::uint(1)]),
            Some(Row::IntAndUint)
        );
        assert_eq!(
            row_of(&[Value::uint(1), Value::int(1)]),
            Some(Row::IntAndUint)
        );
        assert_eq!(
            row_of(&[Value::uint(1), Value::int(1), Value::uint(2)]),
            Some(Row::IntAndUint)
        );
        assert_eq!(
            row_of(&[Value::float(1.0), Value::float(2.0)]),
            Some(Row::Float)
        );
        assert_eq!(row_of(&[Value::bool(true)]), Some(Row::Bool));
        assert_eq!(row_of(&[Value::null(), Value::null()]), Some(Row::Null));
        assert_eq!(
            row_of(&[Value::int(1), Value::uint(1), Value::float(1.0)]),
            None
        );
        // Fewer than two entries takes the fast path whatever they are: there
        // is nothing to compare, which is also what the merge sort does.
        let arr = Value::array(NvsArray::new());
        let mut one = vec![0usize];
        assert!(natural(&mut one, &[arr], false));
        let mut none: Vec<usize> = vec![];
        assert!(natural(&mut none, &[], true));
        dropped(arr);
    }

    /// `sortByKey`'s path: the keys' bytes, both orders, against the merge
    /// sort over the same bytes.
    #[test]
    fn sort_by_key_orders_key_bytes_as_the_merge_sort_does() {
        let mut rng = Lcg(11);
        for &n in SIZES {
            for shape in SHAPES {
                let raw = shaped(&mut rng, n, shape);
                let keys: Vec<NvsStr> = raw
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        let s = if i % 5 == 0 {
                            format!("{}", v % 100)
                        } else {
                            format!("key-{}", v % 5000)
                        };
                        NvsStr::new(s.as_bytes())
                    })
                    .collect();
                for descending in [false, true] {
                    let mut permutation: Vec<usize> = (0..n).collect();
                    by_bytes(&mut permutation, &keys, descending);
                    let mut expected: Vec<usize> = (0..n).collect();
                    let mut compare =
                        |l: usize, r: usize| -> Result<Ordering, nvs_runtime::Fault> {
                            let o = keys[l].as_bytes().cmp(keys[r].as_bytes());
                            Ok(if descending { o.reverse() } else { o })
                        };
                    merge_sort(&mut expected, &mut compare).expect("bytes never throw");
                    assert_eq!(permutation, expected, "{shape}/{n}/desc={descending}");
                }
            }
        }
    }
}
