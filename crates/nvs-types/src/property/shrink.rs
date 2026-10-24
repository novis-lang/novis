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
//! **Memory:** one candidate list per step, per property in flight. A list's candidates copy the
//! list once per shorter form and once per element candidate, so one step is at most a few times
//! the input's length copies of it, and the input is at most
//! [`super::generate::MAX_INPUT_UNITS`] units.

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
            let Some(value) = inputs.get(at) else {
                break;
            };
            for candidate in candidates(value, generator, classes) {
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
#[must_use]
pub fn candidates(value: &Generated, generator: &Gen, classes: &[ClassGen]) -> Vec<Generated> {
    let mut out = match (generator, value) {
        (Gen::Nullable(_), Generated::Null) => Vec::new(),
        (Gen::Nullable(inner), value) => {
            let mut out = vec![Generated::Null];
            out.extend(candidates(value, inner, classes));
            out
        }
        (Gen::Bool, Generated::Bool(true)) => vec![Generated::Bool(false)],
        (Gen::Int { min, max }, Generated::Int(number)) => toward(
            i128::from(*number),
            target(i128::from(*min), i128::from(*max)),
        )
        .into_iter()
        .filter_map(|next| i64::try_from(next).ok())
        .map(Generated::Int)
        .collect(),
        (Gen::Uint { min, max }, Generated::Uint(number)) => toward(
            i128::from(*number),
            target(i128::from(*min), i128::from(*max)),
        )
        .into_iter()
        .filter_map(|next| u64::try_from(next).ok())
        .map(Generated::Uint)
        .collect(),
        (Gen::Float { min, max }, Generated::Float(number)) => floats(*number, *min, *max)
            .into_iter()
            .map(Generated::Float)
            .collect(),
        (Gen::Decimal { min, max }, Generated::Decimal(text)) => decimals(text, *min, *max)
            .into_iter()
            .map(Generated::Decimal)
            .collect(),
        (Gen::Str { len, .. }, Generated::Str(text)) => strings(text, *len)
            .into_iter()
            .map(Generated::Str)
            .collect(),
        (Gen::Bytes { len, .. }, Generated::Bytes(raw)) => bytes(raw, *len),
        (Gen::Enum { values, uint }, Generated::Enum { .. }) => values
            .first()
            .map(|first| Generated::Enum {
                value: *first,
                uint: *uint,
            })
            .into_iter()
            .collect(),
        (Gen::List { element, len }, Generated::List(items)) => {
            lists(items, element, *len, classes)
        }
        (Gen::Shape(fields), Generated::Shape(values)) => shapes(values, fields, classes),
        (Gen::OneOf(arms), value) => unions(value, arms, classes),
        (Gen::Class(label), Generated::Object { class, args }) if class == label => {
            objects(label, args, classes)
        }
        _ => Vec::new(),
    };
    out.retain(|candidate| candidate != value);
    out
}

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

/// The shorter forms of `items`, shortest first, none below `min`: the shortest prefix, the
/// first half, and `items` with one item left out at each position.
fn shorter<T: Clone>(items: &[T], min: usize) -> Vec<Vec<T>> {
    let count = items.len();
    if count <= min {
        return Vec::new();
    }
    let mut out = vec![items[..min].to_vec()];
    let half = count / 2;
    if half > min {
        out.push(items[..half].to_vec());
    }
    for at in 0..count {
        // Leaving out the last item gives a prefix, and that one is already in `out`.
        if at == count - 1 && (count - 1 == min || count - 1 == half) {
            continue;
        }
        let mut kept = items.to_vec();
        kept.remove(at);
        out.push(kept);
    }
    out
}

fn strings(text: &str, len: Len) -> Vec<String> {
    let parts = pieces(text);
    let min = shortest(len);
    let mut out: Vec<String> = shorter(&parts, min)
        .into_iter()
        .map(|kept| kept.concat())
        .collect();
    let other = parts.iter().filter(|part| **part != "a").count();
    if other > 0 {
        out.push("a".repeat(parts.len()));
    }
    if other > 1 {
        for (at, part) in parts.iter().enumerate() {
            if *part != "a" {
                let mut next = parts.clone();
                next[at] = "a";
                out.push(next.concat());
            }
        }
    }
    // A new `a` beside a lone combining mark may join it into one piece, and so shorten it.
    out.retain(|candidate| pieces(candidate).len() >= min);
    out
}

fn bytes(raw: &[u8], len: Len) -> Vec<Generated> {
    let mut out: Vec<Generated> = shorter(raw, shortest(len))
        .into_iter()
        .map(Generated::Bytes)
        .collect();
    if raw.iter().any(|byte| *byte != 0) {
        out.push(Generated::Bytes(vec![0; raw.len()]));
    }
    for (at, byte) in raw.iter().enumerate() {
        for next in toward(i128::from(*byte), 0) {
            let mut changed = raw.to_vec();
            changed[at] = u8::try_from(next).unwrap_or(0);
            out.push(Generated::Bytes(changed));
        }
    }
    out
}

fn lists(items: &[Generated], element: &Gen, len: Len, classes: &[ClassGen]) -> Vec<Generated> {
    let mut out: Vec<Generated> = shorter(items, shortest(len))
        .into_iter()
        .map(Generated::List)
        .collect();
    for (at, item) in items.iter().enumerate() {
        for next in candidates(item, element, classes) {
            let mut changed = items.to_vec();
            changed[at] = next;
            out.push(Generated::List(changed));
        }
    }
    out
}

fn shapes(
    values: &[(String, Generated)],
    fields: &[ShapeGen],
    classes: &[ClassGen],
) -> Vec<Generated> {
    let field = |name: &str| fields.iter().find(|field| field.name == name);
    let mut out = Vec::new();
    for (at, (name, _)) in values.iter().enumerate() {
        if field(name).is_some_and(|field| !field.required) {
            let mut kept = values.to_vec();
            kept.remove(at);
            out.push(Generated::Shape(kept));
        }
    }
    for (at, (name, value)) in values.iter().enumerate() {
        let Some(field) = field(name) else {
            continue;
        };
        for next in candidates(value, &field.generator, classes) {
            let mut changed = values.to_vec();
            changed[at].1 = next;
            out.push(Generated::Shape(changed));
        }
    }
    out
}

fn objects(label: &str, args: &[Generated], classes: &[ClassGen]) -> Vec<Generated> {
    let Some(row) = classes.iter().find(|row| row.label == label) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (at, (arg, generator)) in args.iter().zip(&row.args).enumerate() {
        for next in candidates(arg, generator, classes) {
            let mut changed = args.to_vec();
            changed[at] = next;
            out.push(Generated::Object {
                class: label.to_owned(),
                args: changed,
            });
        }
    }
    out
}

/// A union's candidates: the simplest value of each arm before the first one `value` fits,
/// then the candidates within that arm.
fn unions(value: &Generated, arms: &[Gen], classes: &[ClassGen]) -> Vec<Generated> {
    let Some(at) = arms.iter().position(|arm| fits(value, arm, classes)) else {
        return Vec::new();
    };
    let mut out: Vec<Generated> = arms[..at]
        .iter()
        .filter_map(|arm| simplest(arm, classes, 0))
        .collect();
    out.extend(candidates(value, &arms[at], classes));
    out
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
