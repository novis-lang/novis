//! `rule:types/duration`'s duration grammar,
//! in the one implementation its § 5 requires.
//!
//! `30s`, `1h30m`, `500ms` — Go's `time.ParseDuration` grammar with `d` and `w`
//! added. That ADR's body is the rule; this module is the code, and restates
//! none of the reasoning.
//!
//! # Why the grammar lives in the *syntax* crate
//!
//! § 5 names three places that must agree — the duration written in the source, the run-time
//! `Core\Time\Duration::parse`, and a duration-valued `nvs.toml` directive —
//! and says the three share one parser so a grammar change cannot land in one
//! and miss the others. Exactly one of the three is lexical, so exactly one
//! crate is forced: the lexer cannot reach `nvs-stdlib` without the front end
//! depending on the runtime heap, while `nvs-stdlib` reaching *here* is an edge
//! `rule:tooling/reflection-and-source-parsing-are-core-features`
//! already owes for `Core\Ast`. So this is `nvs-syntax`'s, and the other two
//! call in.
//!
//! # What it produces
//!
//! A count of **nanoseconds** as an `i64`, which is `Core\Time\Duration`'s
//! whole state and its range: ±292 years, Go's own bound, and the one under
//! which `$d->toNanoseconds(): int` is total. Anything wider is
//! [`DurationError::Overflow`] — `rule:types/duration`'s "a literal whose value exceeds
//! `Duration`'s range is a compile error, not a wrap", and the same throw at
//! run time.
//!
//! No sign is accepted anywhere (§ 1), so every value this produces is
//! non-negative; a backwards step is `->minus(7d)` or `->negated()`.

use std::fmt::{self, Write as _};

/// Every way a duration can fail to parse.
///
/// Carries enough to write the whole message without the caller re-deriving
/// anything, because both callers need the *same* sentence: the lexer prints
/// it as a diagnostic and `Core\Time\Duration::parse` throws it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DurationError {
    /// The text is empty.
    Empty,
    /// A unit follows nothing — `ms` on its own, or `1h30` running out before
    /// its unit.
    MissingCount,
    /// A count follows nothing — `1h30` ends with a count and no unit.
    MissingUnit,
    /// A unit spelling this grammar does not have, as written.
    UnknownUnit(String),
    /// A unit written in upper or mixed case — `30S`.
    ///
    /// Its own variant rather than an [`Self::UnknownUnit`] because
    /// `rule:classes/reserved-spellings-are-lower-case` makes "the same word, wrong case" a distinct thing to say, and this
    /// is the message that says it.
    MisCasedUnit(String),
    /// Two units in the wrong order, or the same one twice — `rule:types/duration`'s
    /// "units strictly descend and may not repeat".
    OutOfOrder(&'static str, &'static str),
    /// A character that begins nothing — a `_` separator, a `.`, a sign.
    UnexpectedChar(char),
    /// The total, or one term of it, does not fit `Duration`'s nanosecond
    /// range.
    Overflow,
}

impl DurationError {
    /// The whole sentence, with no leading capital and no trailing period, so
    /// a caller can place it in a diagnostic or a thrown message unchanged.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "a duration is empty".to_string(),
            Self::MissingCount => "a duration unit needs a count before it".to_string(),
            Self::MissingUnit => "a duration count needs a unit after it".to_string(),
            Self::UnknownUnit(unit) => format!(
                "`{unit}` is not a duration unit — the units are `ns`, `us`, `ms`, `s`, `m`, \
                 `h`, `d` and `w`"
            ),
            Self::MisCasedUnit(unit) => format!(
                "a duration unit is lower case only, so `{unit}` is `{}` (`rule:classes/names-resolve-case-sensitively`)",
                unit.to_ascii_lowercase()
            ),
            Self::OutOfOrder(first, second) => format!(
                "duration units strictly descend and may not repeat, so `{second}` may not \
                 follow `{first}`"
            ),
            Self::UnexpectedChar(c) => format!("`{c}` has no meaning in a duration"),
            Self::Overflow => {
                "this duration is longer than `Core\\Time\\Duration` can hold".to_string()
            }
        }
    }
}

/// The units, coarsest first, with each one's length in nanoseconds.
///
/// Order in this table *is* `rule:types/duration`'s descent rule: a unit may only be
/// followed by one later in the table.
const UNITS: &[(&str, i64)] = &[
    ("w", 7 * 24 * 60 * 60 * 1_000_000_000),
    ("d", 24 * 60 * 60 * 1_000_000_000),
    ("h", 60 * 60 * 1_000_000_000),
    ("m", 60 * 1_000_000_000),
    ("s", 1_000_000_000),
    ("ms", 1_000_000),
    ("us", 1_000),
    ("ns", 1),
];

/// Whether `c` can appear in a duration at all — the alphabet the lexer scans
/// a candidate with, so that `30foo` stays an integer followed by an
/// identifier while `30S` reaches [`DurationError::MisCasedUnit`].
///
/// Both cases of every unit letter, because a mis-cased unit has to be
/// *reached* before it can be named.
#[must_use]
pub fn is_duration_char(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(
            c.to_ascii_lowercase(),
            'n' | 'u' | 'm' | 's' | 'h' | 'd' | 'w'
        )
}

/// `text` as a count of nanoseconds, or why it is not one.
///
/// The whole grammar: one or more `count unit` pairs, counts being plain ASCII
/// digits with no separator and no sign, units strictly descending and never
/// repeating.
///
/// # Errors
///
/// A [`DurationError`] naming the first thing wrong, left to right.
pub fn parse(text: &str) -> Result<i64, DurationError> {
    if text.is_empty() {
        return Err(DurationError::Empty);
    }
    let bytes = text.as_bytes();
    let mut at = 0usize;
    let mut total: i64 = 0;
    // The index into `UNITS` of the last unit seen, so the descent rule is one
    // comparison rather than a set of what has been used.
    let mut previous: Option<usize> = None;

    while at < bytes.len() {
        let digits_start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == digits_start {
            // Not a digit, so either a unit with nothing in front of it or a
            // character with no meaning here at all. Which one decides the
            // message, and a letter is the far likelier mistake.
            let c = text[at..].chars().next().expect("at < bytes.len()");
            return Err(if c.is_ascii_alphabetic() {
                DurationError::MissingCount
            } else {
                DurationError::UnexpectedChar(c)
            });
        }
        let count: i64 = text[digits_start..at]
            .parse()
            .map_err(|_| DurationError::Overflow)?;

        let unit_start = at;
        while at < bytes.len() && bytes[at].is_ascii_alphabetic() {
            at += 1;
        }
        if at == unit_start {
            let Some(c) = text[at..].chars().next() else {
                return Err(DurationError::MissingUnit);
            };
            return Err(DurationError::UnexpectedChar(c));
        }
        let unit = &text[unit_start..at];
        let index = UNITS
            .iter()
            .position(|(name, _)| *name == unit)
            .ok_or_else(|| {
                let lowered = unit.to_ascii_lowercase();
                if UNITS.iter().any(|(name, _)| *name == lowered) {
                    DurationError::MisCasedUnit(unit.to_string())
                } else {
                    DurationError::UnknownUnit(unit.to_string())
                }
            })?;
        if let Some(before) = previous
            && index <= before
        {
            return Err(DurationError::OutOfOrder(UNITS[before].0, UNITS[index].0));
        }
        previous = Some(index);

        total = count
            .checked_mul(UNITS[index].1)
            .and_then(|term| total.checked_add(term))
            .ok_or(DurationError::Overflow)?;
    }
    Ok(total)
}

/// `nanos` written back in this grammar, coarsest unit first — what
/// `Core\Time\Duration`'s `Stringable` form emits, so that
/// `parse(render(n)) == n` for every `n`.
///
/// Zero is `"0s"`: the grammar has no empty spelling, and `s` is the unit a
/// reader expects a bare zero in. A negative value is rendered with a leading
/// `-`, which [`parse`] deliberately does **not** accept (`rule:types/duration`) — a
/// negative duration is producible only by `->negated()` or `->minus()`, and
/// showing it as `-1h30m` is more useful than refusing to show it.
#[must_use]
pub fn render(nanos: i64) -> String {
    render_with(nanos, str::to_owned)
}

/// [`render`]'s text handed to `then` from a buffer on the stack, so a caller
/// that copies it once — `Core\Time\Duration::toString` into its `string` —
/// makes that copy the only allocation.
///
/// Every value fits the buffer: the widest count each unit can carry, a sign
/// and every suffix are 33 bytes, which `i64::MIN` reaches.
///
/// # Panics
///
/// Only if a rendering outgrew its 40-byte buffer, which no `i64` does.
pub fn render_with<R>(nanos: i64, then: impl FnOnce(&str) -> R) -> R {
    if nanos == 0 {
        return then("0s");
    }
    let mut out = Fixed {
        bytes: [0; 40],
        len: 0,
    };
    // `unsigned_abs` is the one magnitude `i64::MIN` has: negating it in
    // place would overflow.
    let mut left = nanos.unsigned_abs();
    let signed = if nanos < 0 { "-" } else { "" };
    let mut written = write!(out, "{signed}");
    for (name, length) in UNITS {
        let length = length.unsigned_abs();
        if left >= length {
            written = written.and_then(|()| write!(out, "{}{name}", left / length));
            left %= length;
        }
    }
    written.expect("every duration's rendering fits the buffer");
    then(out.text())
}

/// The stack buffer [`render_with`] writes into.
struct Fixed {
    bytes: [u8; 40],
    len: usize,
}

impl Fixed {
    fn text(&self) -> &str {
        // Only `write_str` fills the buffer, and it copies whole `&str`s.
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

impl fmt::Write for Fixed {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        self.bytes
            .get_mut(self.len..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grammar_accepts_what_adr_0070_writes() {
        assert_eq!(parse("30s"), Ok(30_000_000_000));
        assert_eq!(parse("1h30m"), Ok(5_400_000_000_000));
        assert_eq!(parse("500ms"), Ok(500_000_000));
        assert_eq!(parse("1w"), Ok(604_800_000_000_000));
        assert_eq!(parse("7d"), Ok(604_800_000_000_000));
        assert_eq!(parse("1ns"), Ok(1));
        assert_eq!(parse("1us"), Ok(1_000));
        assert_eq!(parse("0s"), Ok(0));
        assert_eq!(parse("1w2d3h4m5s6ms7us8ns"), Ok(788_645_006_007_008));
    }

    /// Each of `rule:types/duration`'s refusals, by the rule it names.
    #[test]
    fn the_grammar_refuses_what_adr_0070_refuses() {
        assert_eq!(parse("30m1h"), Err(DurationError::OutOfOrder("m", "h")));
        assert_eq!(parse("1h1h"), Err(DurationError::OutOfOrder("h", "h")));
        assert_eq!(parse("30S"), Err(DurationError::MisCasedUnit("S".into())));
        assert_eq!(parse("1H30M"), Err(DurationError::MisCasedUnit("H".into())));
        assert_eq!(parse("1.5s"), Err(DurationError::UnexpectedChar('.')));
        assert_eq!(parse("-7d"), Err(DurationError::UnexpectedChar('-')));
        assert_eq!(parse("1_000ms"), Err(DurationError::UnexpectedChar('_')));
        assert_eq!(parse(""), Err(DurationError::Empty));
        assert_eq!(parse("ms"), Err(DurationError::MissingCount));
        assert_eq!(parse("1h30"), Err(DurationError::MissingUnit));
        assert_eq!(parse("30x"), Err(DurationError::UnknownUnit("x".into())));
    }

    /// The range is `i64` nanoseconds, and the edge of it is an error rather
    /// than a wrap — the property `rule:types/duration` asks the *compiler* to hold.
    #[test]
    fn a_duration_past_the_range_is_an_error_not_a_wrap() {
        assert_eq!(parse("292y"), Err(DurationError::UnknownUnit("y".into())));
        assert_eq!(parse("100000w"), Err(DurationError::Overflow));
        assert_eq!(parse("99999999999999999999s"), Err(DurationError::Overflow));
        assert!(parse("15250w").is_ok());
    }

    /// Every message names the thing that was wrong, so a caller can print it
    /// unchanged — and none of them ends in punctuation a diagnostic would
    /// double up.
    #[test]
    fn every_message_is_a_whole_sentence() {
        for text in ["30m1h", "30S", "1.5s", "", "ms", "1h30", "30x", "100000w"] {
            let message = parse(text).expect_err("this text does not parse").message();
            assert!(!message.is_empty());
            assert!(!message.ends_with('.'));
        }
    }

    /// `rule:types/duration`: a `Duration`'s `Stringable` form re-parses to the value
    /// it came from.
    #[test]
    fn a_rendered_duration_parses_back_to_itself() {
        for nanos in [
            0,
            1,
            999,
            1_000,
            30_000_000_000,
            5_400_000_000_000,
            604_800_000_000_000,
            788_645_006_007_008,
            i64::MAX,
        ] {
            assert_eq!(parse(&render(nanos)), Ok(nanos), "for {nanos}");
        }
    }

    /// A negative duration renders with the sign `parse` refuses, so the two
    /// are not inverses there — deliberately, and this is what holds the
    /// asymmetry visible.
    #[test]
    fn a_negative_duration_renders_with_a_sign_parse_refuses() {
        assert_eq!(render(-5_400_000_000_000), "-1h30m");
        assert_eq!(
            parse("-1h30m"),
            Err(DurationError::UnexpectedChar('-')),
            "the grammar has no sign"
        );
        assert!(render(i64::MIN).starts_with('-'));
    }

    /// The alphabet the lexer scans with covers every unit in both cases and
    /// nothing else — so `30foo` never reaches [`parse`] and `30S` always
    /// does.
    #[test]
    fn the_scanning_alphabet_is_the_units_and_the_digits() {
        for c in "nusmhdwNUSMHDW0123456789".chars() {
            assert!(is_duration_char(c), "{c} should be a duration character");
        }
        for c in "foo._-+xyzXYZ".chars() {
            assert!(!is_duration_char(c), "{c} should not be one");
        }
    }
}
