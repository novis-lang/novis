//! `rule:testing/property-testing`'s values: one call's inputs, generated from a
//! [`super::PropertyCase`] and a seed.
//!
//! **The same seed gives the same inputs.** The generator is a SplitMix64 stream seeded from the
//! run's seed and the call's index, and nothing else is read: no clock, no address, no hash
//! map's order. That is what lets a failure's report name the seed that reproduces it.
//!
//! **Inputs grow over a run.** Call `i` of `n` has a size from 0 to 100, rising with `i`, and a
//! length range is used up to the share of it that size allows. The first calls try the small
//! values most bugs show up at, and the last ones reach the ends of every range. An integer
//! also takes one of its range's edges — the two ends, `0`, `1` and `-1` — about one call in
//! eight.
//!
//! **A string is UTF-8 a person would write, and more.** Each piece of a string is ASCII, a
//! control character, a code point of two, three or four bytes, or one of [`CLUSTERS`]: a
//! letter with combining marks, an emoji sequence joined by U+200D, a flag, a Hangul syllable
//! written as jamo, a Devanagari conjunct and `"\r\n"`. A string's length counts those pieces.
//!
//! **One call's input is bounded.** [`MAX_INPUT_UNITS`] counts every element, byte and string
//! piece of a call's inputs together. A length is cut to what is left of it, and an input whose
//! smallest form does not fit is [`TooLarge`] rather than an allocation.

use super::{ClassGen, Gen, Len, PropertyCase};

/// The most elements, bytes and string pieces one call's inputs may hold together.
pub const MAX_INPUT_UNITS: u64 = 1 << 20;

/// The size the last call of a run reaches.
const MAX_SIZE: u64 = 100;

/// How deep a class, list or shape may nest in one value. Below it, a `?T` is `null`, a list
/// takes its shortest length, and a union takes its first arm that does not nest.
const MAX_DEPTH: u32 = 8;

/// Grapheme clusters of more than one code point (what a person counts as one character).
pub const CLUSTERS: &[&str] = &[
    "e\u{301}",
    "a\u{308}\u{323}",
    "\u{1F469}\u{200D}\u{1F4BB}",
    "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
    "\u{1F1E9}\u{1F1EA}",
    "\u{1F44D}\u{1F3FD}",
    "\u{1100}\u{1161}\u{11A8}",
    "\u{915}\u{94D}\u{937}\u{93F}",
    "\u{E01}\u{E33}",
    "\r\n",
];

/// One generated value, before the runner builds it in a test's isolate.
#[derive(Clone, Debug, PartialEq)]
pub enum Generated {
    /// `null`.
    Null,
    /// A `bool`.
    Bool(bool),
    /// An `int`.
    Int(i64),
    /// A `uint`.
    Uint(u64),
    /// A `float`.
    Float(f64),
    /// A `decimal`, written as its digits: `-12.5`.
    Decimal(String),
    /// A `string`.
    Str(String),
    /// A `bytes` value.
    Bytes(Vec<u8>),
    /// An enum case, by its backing value.
    Enum(i128),
    /// An `array<T>`'s elements, in order.
    List(Vec<Generated>),
    /// An inline shape's fields, in the shape's order. An optional field that was left out is
    /// absent.
    Shape(Vec<(String, Generated)>),
    /// `new <class>(...args)`.
    Object {
        /// The class's label.
        class: String,
        /// The constructor's arguments, in parameter order.
        args: Vec<Generated>,
    },
}

/// One call's inputs would hold more than [`MAX_INPUT_UNITS`], even at their shortest lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooLarge;

/// Call `index`'s arguments of `case`, one per parameter, under `seed`.
///
/// # Errors
///
/// [`TooLarge`] where the inputs' shortest form is over [`MAX_INPUT_UNITS`].
pub fn generate(case: &PropertyCase, seed: u64, index: u32) -> Result<Vec<Generated>, TooLarge> {
    let mut walk = Walk {
        rng: Rng::new(seed ^ u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        size: size(index, case.runs),
        left: MAX_INPUT_UNITS,
        classes: &case.classes,
    };
    case.params
        .iter()
        .map(|generator| walk.value(generator, 0))
        .collect()
}

/// Call `index`'s size, from 0 at the first call to [`MAX_SIZE`] at the last.
fn size(index: u32, runs: u32) -> u64 {
    if runs <= 1 {
        return MAX_SIZE;
    }
    (u64::from(index) * MAX_SIZE / u64::from(runs - 1)).min(MAX_SIZE)
}

/// SplitMix64: a small, fast stream whose output depends on its seed alone.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A stream that starts from `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 random bits.
    pub const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..bound`, for a `bound` above 0.
    pub fn below(&mut self, bound: u64) -> u64 {
        // The widening multiply maps the 64 random bits onto the range with no division.
        let wide = u128::from(self.next()) * u128::from(bound);
        u64::try_from(wide >> 64).unwrap_or(0)
    }

    /// A number in `min..=max`.
    pub fn between_u64(&mut self, min: u64, max: u64) -> u64 {
        match (max - min).checked_add(1) {
            Some(span) => min + self.below(span),
            None => self.next(),
        }
    }

    /// A number in `min..=max`.
    pub fn between_i64(&mut self, min: i64, max: i64) -> i64 {
        let span = max.abs_diff(min);
        let offset = match span.checked_add(1) {
            Some(span) => self.below(span),
            None => self.next(),
        };
        min.wrapping_add_unsigned(offset)
    }

    /// `true` once in `one_in` calls, on average.
    pub fn one_in(&mut self, one_in: u64) -> bool {
        self.below(one_in) == 0
    }
}

/// One call's walk over its generators.
struct Walk<'a> {
    rng: Rng,
    size: u64,
    /// What is left of [`MAX_INPUT_UNITS`].
    left: u64,
    classes: &'a [ClassGen],
}

impl Walk<'_> {
    fn value(&mut self, generator: &Gen, depth: u32) -> Result<Generated, TooLarge> {
        let deep = depth >= MAX_DEPTH;
        Ok(match generator {
            Gen::Literal(value) => value.clone(),
            Gen::Bool => Generated::Bool(self.rng.one_in(2)),
            Gen::Int { min, max } => Generated::Int(self.int(*min, *max)),
            Gen::Uint { min, max } => Generated::Uint(self.uint(*min, *max)),
            Gen::Float { min, max } => Generated::Float(self.float(*min, *max)),
            Gen::Decimal { min, max } => Generated::Decimal(self.decimal(*min, *max)),
            Gen::Str { len, .. } => {
                let count = self.length(*len, deep)?;
                Generated::Str((0..count).fold(String::new(), |mut text, _| {
                    self.piece(&mut text);
                    text
                }))
            }
            Gen::Bytes { len, .. } => {
                let count = self.length(*len, deep)?;
                Generated::Bytes(
                    (0..count)
                        .map(|_| u8::try_from(self.rng.below(256)).unwrap_or(0))
                        .collect(),
                )
            }
            Gen::Enum(values) => {
                let count = u64::try_from(values.len()).unwrap_or(1).max(1);
                let pick = usize::try_from(self.rng.below(count)).unwrap_or(0);
                Generated::Enum(values.get(pick).copied().unwrap_or(0))
            }
            Gen::List { element, len } => {
                let count = self.length(*len, deep)?;
                let mut items = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
                for _ in 0..count {
                    items.push(self.value(element, depth + 1)?);
                }
                Generated::List(items)
            }
            Gen::Shape(fields) => {
                let mut values = Vec::with_capacity(fields.len());
                for field in fields {
                    if !field.required && (deep || self.rng.one_in(4)) {
                        continue;
                    }
                    values.push((field.name.clone(), self.value(&field.generator, depth + 1)?));
                }
                Generated::Shape(values)
            }
            Gen::Nullable(inner) => {
                if deep || self.rng.one_in(8) {
                    Generated::Null
                } else {
                    self.value(inner, depth)?
                }
            }
            Gen::OneOf(arms) => {
                let pick = if deep {
                    arms.iter().position(|arm| !nests(arm)).unwrap_or(0)
                } else {
                    let count = u64::try_from(arms.len()).unwrap_or(1).max(1);
                    usize::try_from(self.rng.below(count)).unwrap_or(0)
                };
                match arms.get(pick) {
                    Some(arm) => self.value(arm, depth)?,
                    None => Generated::Null,
                }
            }
            Gen::Class(label) => {
                let args = self
                    .classes
                    .iter()
                    .find(|class| class.label == *label)
                    .map(|class| class.args.as_slice())
                    .unwrap_or_default();
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.value(arg, depth + 1)?);
                }
                Generated::Object {
                    class: label.clone(),
                    args: values,
                }
            }
        })
    }

    /// A length in `len`, no longer than this call's size allows or [`Self::left`] holds, and
    /// spent from [`Self::left`].
    fn length(&mut self, len: Len, deep: bool) -> Result<u64, TooLarge> {
        let min = u64::from(len.min);
        if min > self.left {
            return Err(TooLarge);
        }
        let reach = u64::from(len.max - len.min) * self.size / MAX_SIZE;
        let max = if deep {
            min
        } else {
            (min + reach).min(self.left)
        };
        let count = self.rng.between_u64(min, max);
        self.left -= count;
        Ok(count)
    }

    fn int(&mut self, min: i64, max: i64) -> i64 {
        if self.rng.one_in(8) {
            let edges = [min, max, 0, 1, -1];
            let edge = edges[usize::try_from(self.rng.below(5)).unwrap_or(0)];
            if (min..=max).contains(&edge) {
                return edge;
            }
        }
        if self.rng.one_in(2) {
            // A value near zero, as far out as the size allows.
            let reach = i64::try_from(self.size * self.size).unwrap_or(i64::MAX);
            let (low, high) = (min.max(-reach), max.min(reach));
            if low <= high {
                return self.rng.between_i64(low, high);
            }
        }
        self.rng.between_i64(min, max)
    }

    fn uint(&mut self, min: u64, max: u64) -> u64 {
        if self.rng.one_in(8) {
            let edges = [min, max, 0, 1];
            let edge = edges[usize::try_from(self.rng.below(4)).unwrap_or(0)];
            if (min..=max).contains(&edge) {
                return edge;
            }
        }
        if self.rng.one_in(2) {
            let high = max.min(self.size * self.size);
            if min <= high {
                return self.rng.between_u64(min, high);
            }
        }
        self.rng.between_u64(min, max)
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "a random fraction needs 53 bits of the 64, and the rest are dropped"
    )]
    fn float(&mut self, min: f64, max: f64) -> f64 {
        if self.rng.one_in(8) {
            let edges = [min, max, 0.0, -0.0, 1.0, -1.0];
            let edge = edges[usize::try_from(self.rng.below(6)).unwrap_or(0)];
            if min <= edge && edge <= max {
                return edge;
            }
        }
        let unit = (self.rng.next() >> 11) as f64 / (1u64 << 53) as f64;
        if self.rng.one_in(2) {
            let reach = (self.size * self.size) as f64;
            let (low, high) = (min.max(-reach), max.min(reach));
            if low <= high {
                return (high - low).mul_add(unit, low);
            }
        }
        // Each half on its own, so `max - min` cannot overflow to infinity.
        let value = (max / 2.0 - min / 2.0).mul_add(unit, min / 2.0) * 2.0;
        value.clamp(min, max)
    }

    fn decimal(&mut self, min: i64, max: i64) -> String {
        let whole = self.int(min, max);
        let places = self.rng.below(5);
        // A fraction on either end would take the value out of the range.
        if places == 0 || whole == max || whole == min {
            return whole.to_string();
        }
        let digits: String = (0..places)
            .map(|_| char::from(b'0' + u8::try_from(self.rng.below(10)).unwrap_or(0)))
            .collect();
        // A negative whole part keeps its sign, and so does a zero one below a negative `min`.
        let sign = if whole == 0 && min < 0 && self.rng.one_in(2) {
            "-"
        } else {
            ""
        };
        format!("{sign}{whole}.{digits}")
    }

    /// One piece of a string, appended to `text`.
    fn piece(&mut self, text: &mut String) {
        let code = match self.rng.below(20) {
            0..10 => u32::try_from(self.rng.between_u64(0x20, 0x7E)).unwrap_or(0x20),
            10 => [0x0A, 0x09, 0x00, 0x7F][usize::try_from(self.rng.below(4)).unwrap_or(0)],
            11 | 12 => u32::try_from(self.rng.between_u64(0x80, 0x7FF)).unwrap_or(0x80),
            13 | 14 => {
                // Three bytes, with the surrogate block left out: it is not UTF-8.
                let code = u32::try_from(self.rng.between_u64(0x800, 0xF7FF)).unwrap_or(0x800);
                if code >= 0xD800 { code + 0x800 } else { code }
            }
            15 => u32::try_from(self.rng.between_u64(0x1_0000, 0x10_FFFF)).unwrap_or(0x1_0000),
            _ => {
                let pick = self.rng.below(u64::try_from(CLUSTERS.len()).unwrap_or(1));
                text.push_str(CLUSTERS[usize::try_from(pick).unwrap_or(0)]);
                return;
            }
        };
        text.push(char::from_u32(code).unwrap_or('?'));
    }
}

/// Whether `generator` builds a value that holds other generated values.
fn nests(generator: &Gen) -> bool {
    matches!(
        generator,
        Gen::List { .. } | Gen::Shape(_) | Gen::Class(_) | Gen::OneOf(_)
    ) || matches!(generator, Gen::Nullable(inner) if nests(inner))
}
