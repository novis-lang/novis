//! `rule:testing/property-testing`'s shrinker: a failing call's inputs, made simpler one step at
//! a time until no simpler input still fails.
//!
//! **Every candidate is a value its generator could have made.** [`candidates`] reads the same
//! [`Gen`] the inputs came from, so a number stays in its `gen:` range, a list, string or `bytes`
//! keeps at least its shortest length, and an enum stays one of its cases. A test is never
//! called with an input its declared type does not allow, and [`fits`] is that check.
//!
//! **Simpler comes first.** A `?T` tries `null`. A list, string or `bytes` tries shorter forms
//! before it shrinks what it contains, and a string's pieces then become `a`. A number moves
//! toward `0`, or toward the end of its range nearest `0`, by halving the distance, and a float
//! also tries its whole part. A `bool` becomes `false` and an enum its first case. A shape drops
//! an optional field before it shrinks one, an object shrinks its constructor's arguments, and a
//! union tries the simplest value of each earlier arm before it shrinks within its own.
//!
//! **A string is cut by pieces.** [`pieces`] splits it the way the generator counts its length:
//! one of [`CLUSTERS`] is one piece, and every other code point is one. A cut never splits a
//! code point, so every candidate is valid UTF-8.
//!
//! **The loop is greedy and bounded.** [`shrink`] takes the first candidate that still fails,
//! starts again from it, and stops when no candidate fails or `budget` calls are spent.
//! [`MAX_SHRINK_CALLS`] is the budget a runner uses when nothing else is said. Nothing is random,
//! so one failing input always shrinks to the same counterexample.
//!
//! **A candidate is built when it is tried.** [`candidates`] is an iterator, so a step that
//! keeps its first candidate builds one, and a shrink builds at most `budget` candidates. A list
//! of `n` items has about `n` shorter forms of `n` items each, so a list of them would cost `n²`
//! units of time and memory in every step.
//!
//! **Memory:** one candidate and one copy of the parameter it replaces, per property in flight,
//! and each is at most [`super::generate::MAX_INPUT_UNITS`] units.

use std::iter;

use super::generate::{CLUSTERS, Generated};
use super::{ClassGen, Gen, Len, ShapeGen};

/// The most calls of a property [`shrink`] makes when the runner gives no other budget.
pub const MAX_SHRINK_CALLS: u32 = 1000;

/// How many halving steps a float takes toward its target. A float never reaches its target by
/// halving alone, so the steps stop here.
const FLOAT_STEPS: u32 = 16;

/// How deep [`simplest`] builds a union arm's value before it gives up on that arm.
const SIMPLEST_DEPTH: u32 = 4;

/// The inputs `inputs`, shrunk while `fails` still returns `true` for them, and how many calls
/// of `fails` that took.
///
/// One parameter is shrunk at a time, in order. `fails` gets the whole argument list with that
/// parameter replaced by one of its [`candidates`]. The first candidate that fails is kept and
/// the walk starts again from the first parameter. The loop ends when no candidate of any
/// parameter fails, or after `budget` calls.
#[must_use]
pub fn shrink(
    mut inputs: Vec<Generated>,
    params: &[Gen],
    classes: &[ClassGen],
    mut fails: impl FnMut(&[Generated]) -> bool,
    budget: u32,
) -> (Vec<Generated>, u32) {
    let mut calls = 0;
    'step: loop {
        for (at, generator) in params.iter().enumerate() {
            // A copy, because the candidates are built from it while `inputs[at]` changes.
            let Some(value) = inputs.get(at).cloned() else {
                break;
            };
            for candidate in candidates(&value, generator, classes) {
                if calls >= budget {
                    break 'step;
                }
                calls += 1;
                let before = std::mem::replace(&mut inputs[at], candidate);
                if fails(&inputs) {
                    continue 'step;
                }
                inputs[at] = before;
            }
        }
        break;
    }
    (inputs, calls)
}

/// The values simpler than `value` that `generator` could have made, simplest first. `value`
/// itself is never among them.
///
/// Each candidate is built when the walk reaches it, so the walk holds one candidate at a time.
pub fn candidates<'a>(
    value: &'a Generated,
    generator: &'a Gen,
    classes: &'a [ClassGen],
) -> Candidates<'a> {
    let out: Candidates<'a> = match (generator, value) {
        (Gen::Nullable(_), Generated::Null) => Box::new(iter::empty()),
        (Gen::Nullable(inner), value) => {
            Box::new(iter::once(Generated::Null).chain(candidates(value, inner, classes)))
        }
        (Gen::Bool, Generated::Bool(true)) => Box::new(iter::once(Generated::Bool(false))),
        (Gen::Int { min, max }, Generated::Int(number)) => Box::new(
            toward(
                i128::from(*number),
                target(i128::from(*min), i128::from(*max)),
            )
            .into_iter()
            .filter_map(|next| i64::try_from(next).ok())
            .map(Generated::Int),
        ),
        (Gen::Uint { min, max }, Generated::Uint(number)) => Box::new(
            toward(
                i128::from(*number),
                target(i128::from(*min), i128::from(*max)),
            )
            .into_iter()
            .filter_map(|next| u64::try_from(next).ok())
            .map(Generated::Uint),
        ),
        (Gen::Float { min, max }, Generated::Float(number)) => Box::new(
            floats(*number, *min, *max)
                .into_iter()
                .map(Generated::Float),
        ),
        (Gen::Decimal { min, max }, Generated::Decimal(text)) => Box::new(
            decimals(text, *min, *max)
                .into_iter()
                .map(Generated::Decimal),
        ),
        (Gen::Str { len, .. }, Generated::Str(text)) => {
            Box::new(strings(text, *len).map(Generated::Str))
        }
        (Gen::Bytes { len, .. }, Generated::Bytes(raw)) => Box::new(bytes(raw, *len)),
        (Gen::Enum { values, uint }, Generated::Enum { .. }) => Box::new(
            values
                .first()
                .map(|first| Generated::Enum {
                    value: *first,
                    uint: *uint,
                })
                .into_iter(),
        ),
        (Gen::List { element, len }, Generated::List(items)) => {
            Box::new(lists(items, element, *len, classes))
        }
        (Gen::Shape(fields), Generated::Shape(values)) => Box::new(shapes(values, fields, classes)),
        (Gen::OneOf(arms), value) => unions(value, arms, classes),
        (Gen::Class(label), Generated::Object { class, args }) if class == label => {
            objects(label, args, classes)
        }
        _ => Box::new(iter::empty()),
    };
    Box::new(out.filter(move |candidate| candidate != value))
}

/// The candidates of one value, in [`candidates`]' order.
pub type Candidates<'a> = Box<dyn Iterator<Item = Generated> + 'a>;

/// Whether `generator` could have made `value`: the same kind, every number in its range,
/// every length in its range, and the same for each value it contains.
#[must_use]
pub fn fits(value: &Generated, generator: &Gen, classes: &[ClassGen]) -> bool {
    match (generator, value) {
        (Gen::Literal(literal), value) => literal == value,
        (Gen::Bool, Generated::Bool(_)) | (Gen::Nullable(_), Generated::Null) => true,
        (Gen::Int { min, max }, Generated::Int(number)) => (min..=max).contains(&number),
        (Gen::Uint { min, max }, Generated::Uint(number)) => (min..=max).contains(&number),
        (Gen::Float { min, max }, Generated::Float(number)) => {
            number.is_finite() && min <= number && number <= max
        }
        (Gen::Decimal { min, max }, Generated::Decimal(text)) => decimal_fits(text, *min, *max),
        (Gen::Str { len, .. }, Generated::Str(text)) => within(*len, pieces(text).len()),
        (Gen::Bytes { len, .. }, Generated::Bytes(raw)) => within(*len, raw.len()),
        (Gen::Enum { values, .. }, Generated::Enum { value, .. }) => values.contains(value),
        (Gen::List { element, len }, Generated::List(items)) => {
            within(*len, items.len()) && items.iter().all(|item| fits(item, element, classes))
        }
        (Gen::Shape(fields), Generated::Shape(values)) => {
            values.iter().all(|(name, value)| {
                fields
                    .iter()
                    .find(|field| field.name == *name)
                    .is_some_and(|field| fits(value, &field.generator, classes))
            }) && fields
                .iter()
                .filter(|field| field.required)
                .all(|field| values.iter().any(|(name, _)| *name == field.name))
        }
        (Gen::Nullable(inner), value) => fits(value, inner, classes),
        (Gen::OneOf(arms), value) => arms.iter().any(|arm| fits(value, arm, classes)),
        (Gen::Class(label), Generated::Object { class, args }) => {
            class == label
                && classes
                    .iter()
                    .find(|row| row.label == *label)
                    .is_some_and(|row| {
                        row.args.len() == args.len()
                            && args
                                .iter()
                                .zip(&row.args)
                                .all(|(arg, generator)| fits(arg, generator, classes))
                    })
        }
        _ => false,
    }
}

/// `text` split into the pieces a string's length counts: one of [`CLUSTERS`], or one code
/// point.
#[must_use]
pub fn pieces(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(first) = rest.chars().next() {
        let width = CLUSTERS
            .iter()
            .filter(|cluster| rest.starts_with(**cluster))
            .map(|cluster| cluster.len())
            .max()
            .unwrap_or_else(|| first.len_utf8());
        let (piece, tail) = rest.split_at(width);
        out.push(piece);
        rest = tail;
    }
    out
}

/// Whether a length of `count` is in `len`.
fn within(len: Len, count: usize) -> bool {
    u32::try_from(count).is_ok_and(|count| len.min <= count && count <= len.max)
}

/// `len`'s shortest length, as a count of items.
fn shortest(len: Len) -> usize {
    usize::try_from(len.min).unwrap_or(usize::MAX)
}

/// The number in `min..=max` a number shrinks toward: `0`, or the end nearest it.
const fn target(min: i128, max: i128) -> i128 {
    if min > 0 {
        min
    } else if max < 0 {
        max
    } else {
        0
    }
}

/// The numbers between `goal` and `value` a number tries, simplest first: `goal`, then `value`
/// moved half the way, a quarter of the way, and so on down to one step.
fn toward(value: i128, goal: i128) -> Vec<i128> {
    if value == goal {
        return Vec::new();
    }
    let mut out = vec![goal];
    let mut step = (value - goal) / 2;
    while step != 0 {
        out.push(value - step);
        step /= 2;
    }
    out
}

#[allow(
    clippy::float_cmp,
    reason = "a candidate equal to the value is not simpler, and only exact equality says so"
)]
fn floats(value: f64, min: f64, max: f64) -> Vec<f64> {
    let goal = if min > 0.0 {
        min
    } else if max < 0.0 {
        max
    } else {
        0.0
    };
    let mut out = vec![goal];
    if !value.is_finite() {
        return out;
    }
    let whole = value.trunc();
    if whole != value && min <= whole && whole <= max {
        out.push(whole);
    }
    let mut distance = value - goal;
    for _ in 0..FLOAT_STEPS {
        distance /= 2.0;
        let next = (value - distance).clamp(min, max);
        if next == value {
            break;
        }
        out.push(next);
    }
    out
}

/// A decimal's whole part as written, with its sign, and its digits after the point.
fn decimal_parts(text: &str) -> (&str, &str) {
    text.split_once('.').unwrap_or((text, ""))
}

fn decimal_fits(text: &str, min: i64, max: i64) -> bool {
    let (whole_text, fraction) = decimal_parts(text);
    let negative = whole_text.starts_with('-');
    whole_text.parse::<i64>().is_ok_and(|whole| {
        // A fraction past either end takes the value out of the range: `max.5` or `-min.5`.
        let past_an_end =
            !fraction.is_empty() && ((whole == max && !negative) || (whole == min && negative));
        (min..=max).contains(&whole) && !past_an_end
    })
}

fn decimals(text: &str, min: i64, max: i64) -> Vec<String> {
    let (whole_text, fraction) = decimal_parts(text);
    let Ok(whole) = whole_text.parse::<i64>() else {
        return Vec::new();
    };
    let goal = target(i128::from(min), i128::from(max));
    let mut out = vec![goal.to_string()];
    if !fraction.is_empty() {
        out.push(whole.to_string());
    }
    out.extend(
        toward(i128::from(whole), goal)
            .into_iter()
            .skip(1)
            .map(|next| next.to_string()),
    );
    // One digit fewer after the point keeps the value between its whole part and itself.
    if let Some((cut, _)) = fraction.char_indices().last()
        && cut > 0
    {
        out.push(format!("{whole_text}.{}", &fraction[..cut]));
    }
    out.dedup();
    out
}

/// One shorter form of a list of `count` items: a prefix of that length, or the list with the
/// item at that position left out.
#[derive(Clone, Copy)]
enum Cut {
    Prefix(usize),
    Without(usize),
}

impl Cut {
    fn apply<T: Clone>(self, items: &[T]) -> Vec<T> {
        match self {
            Self::Prefix(end) => items[..end].to_vec(),
            Self::Without(at) => {
                let mut kept = Vec::with_capacity(items.len() - 1);
                kept.extend_from_slice(&items[..at]);
                kept.extend_from_slice(&items[at + 1..]);
                kept
            }
        }
    }
}

/// The shorter forms of a list of `count` items, shortest first, none below `min`: the shortest
/// prefix, the first half, and the list with one item left out at each position.
fn cuts(count: usize, min: usize) -> impl Iterator<Item = Cut> {
    let half = count / 2;
    let some = count > min;
    let prefixes = [Some(min), (half > min).then_some(half)]
        .into_iter()
        .flatten()
        .filter(move |_| some)
        .map(Cut::Prefix);
    // Leaving out the last item gives a prefix, and that one is already among `prefixes`.
    let without = (0..count)
        .filter(move |at| some && !(*at == count - 1 && (count - 1 == min || count - 1 == half)))
        .map(Cut::Without);
    prefixes.chain(without)
}

fn strings(text: &str, len: Len) -> impl Iterator<Item = String> + '_ {
    let parts = pieces(text);
    let min = shortest(len);
    let count = parts.len();
    let other = parts.iter().filter(|part| **part != "a").count();
    let shorter = {
        let parts = parts.clone();
        cuts(count, min).map(move |cut| cut.apply(&parts).concat())
    };
    let all_a = (other > 0).then(|| "a".repeat(count));
    let one_a = (0..count).filter(move |_| other > 1).filter_map(move |at| {
        if parts[at] == "a" {
            return None;
        }
        let mut next = parts.clone();
        next[at] = "a";
        Some(next.concat())
    });
    // A new `a` beside a lone combining mark may join it into one piece, and so shorten it.
    shorter
        .chain(all_a)
        .chain(one_a)
        .filter(move |candidate| pieces(candidate).len() >= min)
}

fn bytes(raw: &[u8], len: Len) -> impl Iterator<Item = Generated> + '_ {
    let shorter = cuts(raw.len(), shortest(len)).map(|cut| Generated::Bytes(cut.apply(raw)));
    let zeros = raw
        .iter()
        .any(|byte| *byte != 0)
        .then(|| Generated::Bytes(vec![0; raw.len()]));
    let smaller = raw.iter().enumerate().flat_map(move |(at, byte)| {
        toward(i128::from(*byte), 0).into_iter().map(move |next| {
            let mut changed = raw.to_vec();
            changed[at] = u8::try_from(next).unwrap_or(0);
            Generated::Bytes(changed)
        })
    });
    shorter.chain(zeros).chain(smaller)
}

fn lists<'a>(
    items: &'a [Generated],
    element: &'a Gen,
    len: Len,
    classes: &'a [ClassGen],
) -> impl Iterator<Item = Generated> + 'a {
    let shorter = cuts(items.len(), shortest(len)).map(|cut| Generated::List(cut.apply(items)));
    let simpler = items.iter().enumerate().flat_map(move |(at, item)| {
        candidates(item, element, classes).map(move |next| {
            let mut changed = items.to_vec();
            changed[at] = next;
            Generated::List(changed)
        })
    });
    shorter.chain(simpler)
}

fn shapes<'a>(
    values: &'a [(String, Generated)],
    fields: &'a [ShapeGen],
    classes: &'a [ClassGen],
) -> impl Iterator<Item = Generated> + 'a {
    let field = move |name: &str| fields.iter().find(|field| field.name == name);
    let dropped = values
        .iter()
        .enumerate()
        .filter(move |(_, (name, _))| field(name).is_some_and(|field| !field.required))
        .map(move |(at, _)| {
            let mut kept = values.to_vec();
            kept.remove(at);
            Generated::Shape(kept)
        });
    let simpler = values
        .iter()
        .enumerate()
        .filter_map(move |(at, (name, value))| field(name).map(|field| (at, value, field)))
        .flat_map(move |(at, value, field)| {
            candidates(value, &field.generator, classes).map(move |next| {
                let mut changed = values.to_vec();
                changed[at].1 = next;
                Generated::Shape(changed)
            })
        });
    dropped.chain(simpler)
}

fn objects<'a>(label: &'a str, args: &'a [Generated], classes: &'a [ClassGen]) -> Candidates<'a> {
    let Some(row) = classes.iter().find(|row| row.label == label) else {
        return Box::new(iter::empty());
    };
    Box::new(
        args.iter()
            .zip(&row.args)
            .enumerate()
            .flat_map(move |(at, (arg, generator))| {
                candidates(arg, generator, classes).map(move |next| {
                    let mut changed = args.to_vec();
                    changed[at] = next;
                    Generated::Object {
                        class: label.to_owned(),
                        args: changed,
                    }
                })
            }),
    )
}

/// A union's candidates: the simplest value of each arm before the first one `value` fits,
/// then the candidates within that arm.
fn unions<'a>(value: &'a Generated, arms: &'a [Gen], classes: &'a [ClassGen]) -> Candidates<'a> {
    let Some(at) = arms.iter().position(|arm| fits(value, arm, classes)) else {
        return Box::new(iter::empty());
    };
    Box::new(
        arms[..at]
            .iter()
            .filter_map(move |arm| simplest(arm, classes, 0))
            .chain(candidates(value, &arms[at], classes)),
    )
}

/// The simplest value `generator` makes, where one is quick to build. A list that may not be
/// empty and anything nested deeper than [`SIMPLEST_DEPTH`] has none.
fn simplest(generator: &Gen, classes: &[ClassGen], depth: u32) -> Option<Generated> {
    if depth > SIMPLEST_DEPTH {
        return None;
    }
    let goal = |min: i64, max: i64| target(i128::from(min), i128::from(max));
    Some(match generator {
        Gen::Literal(value) => value.clone(),
        Gen::Bool => Generated::Bool(false),
        Gen::Int { min, max } => Generated::Int(i64::try_from(goal(*min, *max)).ok()?),
        Gen::Uint { min, max } => {
            Generated::Uint(u64::try_from(target(i128::from(*min), i128::from(*max))).ok()?)
        }
        Gen::Float { min, max } => Generated::Float(*floats(0.0, *min, *max).first()?),
        Gen::Decimal { min, max } => Generated::Decimal(goal(*min, *max).to_string()),
        Gen::Str { len, .. } => Generated::Str("a".repeat(usize::try_from(len.min).ok()?)),
        Gen::Bytes { len, .. } => Generated::Bytes(vec![0; usize::try_from(len.min).ok()?]),
        Gen::Enum { values, uint } => Generated::Enum {
            value: *values.first()?,
            uint: *uint,
        },
        Gen::List { len, .. } if len.min == 0 => Generated::List(Vec::new()),
        Gen::List { .. } => return None,
        Gen::Shape(fields) => Generated::Shape(
            fields
                .iter()
                .filter(|field| field.required)
                .map(|field| {
                    simplest(&field.generator, classes, depth + 1)
                        .map(|value| (field.name.clone(), value))
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Gen::Nullable(_) => Generated::Null,
        Gen::OneOf(arms) => return simplest(arms.first()?, classes, depth + 1),
        Gen::Class(label) => {
            let row = classes.iter().find(|row| row.label == *label)?;
            Generated::Object {
                class: label.clone(),
                args: row
                    .args
                    .iter()
                    .map(|arg| simplest(arg, classes, depth + 1))
                    .collect::<Option<Vec<_>>>()?,
            }
        }
    })
}
