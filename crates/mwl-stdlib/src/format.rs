//! `Core\Str::format`'s `printf` grammar —
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 1's *Transformation* table, and the one `Core` member that reads a
//! template rather than an option.
//!
//! # Why the grammar is `printf`'s, and why it is written here
//!
//! The spec keeps PHP's `printf` grammar deliberately, so this is a *closed*
//! conversion list (`%s %d %u %f %e %g %x %X %o %b %%`) with `printf`'s flag,
//! width, precision and `%1$s` positional syntax, minus everything that reads
//! ambient state. Nothing outside is accepted: an unknown conversion character
//! throws rather than being copied through, which is the difference between a
//! typo that is reported and one that silently ships.
//!
//! No crate implements it. The `printf`-alike crates on crates.io format Rust
//! values or re-expose C's `vsnprintf`; what is needed here is the grammar
//! applied to a [`Value`] — a tagged union whose conversion rows are
//! [ADR 0007](../../../../docs/adr/0007-explicit-type-system.md) § 2's and
//! [ADR 0054](../../../../docs/adr/0054-decimal-scalar-type.md) § 4's, not
//! Rust's `Display`. Adapting one would be more code than the grammar, so
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 4's first
//! question answers itself: this is MWL's own semantics, not an external
//! specification someone else maintains.
//!
//! # What it refuses, and why each is a throw
//!
//! [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R4 makes
//! failure a throw, and PHP's `printf` argument-mismatch bug family is exactly
//! what the spec wants turned into an error:
//!
//! * **Too few arguments** for the placeholders written — PHP 8 throws here
//!   too, so this is compatibility rather than divergence.
//! * **An argument no placeholder consumed.** PHP ignores a trailing extra;
//!   here it throws, because an unread argument is a mismatch in the same
//!   family and silence is what made that family a bug family. Positional
//!   placeholders make it a real rule rather than a count: every argument must
//!   be named by at least one placeholder, in any order and any number of
//!   times.
//! * **A value with no reading for its conversion** — an array for `%d`, a
//!   non-integral `decimal` for `%x`. The `decimal` row is ADR 0054 § 4's,
//!   which makes rounding something the program says out loud.
//!
//! # Still owed
//!
//! The spec makes `format` an
//! [ADR 0057](../../../../docs/adr/0057-intrinsic-literal-folding.md)
//! intrinsic: a *literal* template should have its placeholder count and types
//! checked while compiling. That machinery does not exist yet (the same one
//! ADR 0056 § 3's compile-time regex tiering waits on), so today every one of
//! the refusals above is a throw at the call rather than a diagnostic before
//! it. Nothing about this module changes when it lands — the checker gains a
//! pass that answers earlier.
//!
//! # Width and precision count what `Core\Str::length` counts
//!
//! `%10s` pads to ten of [`crate::granularity::DEFAULT`]'s units and `%.2s`
//! truncates to two of them, so `format` measures a `string` the way
//! `length`, `at` and `padStart` do rather than the way C counts bytes. ADR
//! 0009 § 2 owns that unit; restating it per member is what would let two
//! members drift apart.

use mwl_runtime::{Fault, MwlStr, Tag, Value};

use crate::granularity::DEFAULT;

/// `template` with every placeholder replaced from `arguments` — the whole of
/// `Core\Str::format`, and the only entry point.
///
/// # Errors
///
/// A `Fault::thrown` for every refusal this module's own docs list: a
/// malformed or unknown placeholder, an argument count that does not match the
/// placeholders, or a value with no reading for the conversion it reached.
pub(crate) fn format(template: &str, arguments: &[Value]) -> Result<String, Fault> {
    let mut out = String::with_capacity(template.len());
    let mut used = vec![false; arguments.len()];
    let mut next = 0usize;
    let mut rest = template;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        if let Some(tail) = after.strip_prefix('%') {
            out.push('%');
            rest = tail;
            continue;
        }
        let (spec, tail) = Spec::parse(after)?;
        let index = match spec.argnum {
            Some(argnum) => argnum - 1,
            None => {
                let index = next;
                next += 1;
                index
            }
        };
        let argument = arguments.get(index).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Str::format(): the template reads argument {} of {}",
                index + 1,
                arguments.len()
            ))
        })?;
        used[index] = true;
        let body = spec.convert(argument)?;
        spec.pad_into(&body, &mut out);
        rest = tail;
    }
    out.push_str(rest);
    if let Some(unused) = used.iter().position(|seen| !seen) {
        return Err(Fault::thrown(format!(
            "Core\\Str::format(): argument {} is never read by the template",
            unused + 1
        )));
    }
    Ok(out)
}

/// One parsed placeholder: `%[argnum$][flags][width][.precision]conversion`.
struct Spec {
    /// The 1-based argument `%1$s` names, or `None` for the next unnamed one.
    argnum: Option<usize>,
    /// The `-` flag: pad on the right rather than the left.
    left: bool,
    /// The `+` flag: write a `+` before a non-negative number.
    plus: bool,
    /// What padding is made of — a space, `0` for the `0` flag, or the
    /// character after `'`.
    pad: char,
    /// The minimum rendered width, in [`DEFAULT`]'s units.
    width: usize,
    /// `.n`: significant digits for `%g`, digits after the point for
    /// `%f`/`%e`, a maximum length for `%s`, and nothing at all for the
    /// integer conversions.
    precision: Option<usize>,
    /// The conversion character, already checked against the closed list.
    conversion: char,
}

impl Spec {
    /// Parses one placeholder off the front of `after` — the text following
    /// the `%`, with `%%` already handled by the caller — and returns it with
    /// whatever follows it.
    fn parse(after: &str) -> Result<(Self, &str), Fault> {
        let bytes = after.as_bytes();
        let mut at = 0usize;

        // `1$` is an argument number only when the `$` is actually there —
        // `%12d` is a width, so the digits are left for the width scan below
        // when it is not. Reading ahead rather than committing is the whole
        // reason this is written over byte indices.
        let mut argnum = None;
        let mut digits = 0usize;
        while bytes.get(digits).is_some_and(u8::is_ascii_digit) {
            digits += 1;
        }
        if digits > 0 && bytes.get(digits) == Some(&b'$') {
            let parsed = after[..digits].parse::<usize>().ok().filter(|n| *n > 0);
            argnum = Some(parsed.ok_or_else(|| malformed(after))?);
            at = digits + 1;
        }

        let mut left = false;
        let mut plus = false;
        let mut pad = ' ';
        loop {
            match bytes.get(at) {
                Some(b'-') => left = true,
                Some(b'+') => plus = true,
                // PHP's space flag asks for the padding that is already the
                // default, so it is accepted and means nothing.
                Some(b' ') => {}
                Some(b'0') => pad = '0',
                Some(b'\'') => {
                    let c = after[at + 1..]
                        .chars()
                        .next()
                        .ok_or_else(|| malformed(after))?;
                    pad = c;
                    at += 1 + c.len_utf8();
                    continue;
                }
                _ => break,
            }
            at += 1;
        }

        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        let width = match at == start {
            true => 0,
            false => after[start..at]
                .parse::<usize>()
                .map_err(|_| malformed(after))?,
        };

        let mut precision = None;
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            let start = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            // A bare `.` is `.0`, which is C's own reading.
            precision = Some(match at == start {
                true => 0,
                false => after[start..at]
                    .parse::<usize>()
                    .map_err(|_| malformed(after))?,
            });
        }

        let conversion = after[at..].chars().next().ok_or_else(|| malformed(after))?;
        at += conversion.len_utf8();
        if !matches!(
            conversion,
            's' | 'd' | 'u' | 'f' | 'e' | 'g' | 'x' | 'X' | 'o' | 'b'
        ) {
            return Err(Fault::thrown(format!(
                "Core\\Str::format(): `%{conversion}` is not one of the conversions the template \
                 grammar allows (`%s %d %u %f %e %g %x %X %o %b %%`)"
            )));
        }
        Ok((
            Self {
                argnum,
                left,
                plus,
                pad,
                width,
                precision,
                conversion,
            },
            &after[at..],
        ))
    }

    /// This placeholder's argument rendered, before any padding.
    fn convert(&self, argument: &Value) -> Result<String, Fault> {
        match self.conversion {
            's' => {
                let rendered = rendered(argument)?;
                Ok(match self.precision {
                    Some(precision) if precision < DEFAULT.length(&rendered) => {
                        rendered[..DEFAULT.byte_of_index(&rendered, precision)].to_owned()
                    }
                    _ => rendered,
                })
            }
            'd' => Ok(self.signed(integer(argument, 'd')?)),
            // PHP's `%u` prints the two's-complement reading of a negative
            // `int`, which is the whole point of having it beside `%d`.
            #[expect(
                clippy::cast_sign_loss,
                reason = "the reinterpretation is what `%u` asks for"
            )]
            'u' => Ok((integer(argument, 'u')? as u64).to_string()),
            #[expect(
                clippy::cast_sign_loss,
                reason = "a base conversion is over the bit pattern, as PHP's is"
            )]
            'x' | 'X' | 'o' | 'b' => {
                let bits = integer(argument, self.conversion)? as u64;
                Ok(match self.conversion {
                    'x' => format!("{bits:x}"),
                    'X' => format!("{bits:X}"),
                    'o' => format!("{bits:o}"),
                    _ => format!("{bits:b}"),
                })
            }
            'f' => {
                Ok(self.signed_text(fixed(floating(argument, 'f')?, self.precision.unwrap_or(6))))
            }
            'e' => Ok(self.signed_text(scientific(
                floating(argument, 'e')?,
                self.precision.unwrap_or(6),
            ))),
            _ => Ok(self.signed_text(shortest(
                floating(argument, 'g')?,
                self.precision.unwrap_or(6),
            ))),
        }
    }

    /// One integer with the `+` flag applied.
    fn signed(&self, value: i64) -> String {
        match self.plus && value >= 0 {
            true => format!("+{value}"),
            false => value.to_string(),
        }
    }

    /// The same, for a float already rendered — its own sign is already in the
    /// text, so only a missing `+` is added.
    fn signed_text(&self, text: String) -> String {
        match self.plus && !text.starts_with('-') {
            true => format!("+{text}"),
            false => text,
        }
    }

    /// `body` padded to [`Self::width`] and appended to `out`.
    ///
    /// Zero padding goes *after* the sign — `%08.3f` of `-3.14159` is
    /// `-003.142`, not `00-3.142` — which is the one place the pad character
    /// changes where the padding lands.
    fn pad_into(&self, body: &str, out: &mut String) {
        let length = DEFAULT.length(body);
        let Some(short) = self.width.checked_sub(length).filter(|short| *short > 0) else {
            out.push_str(body);
            return;
        };
        if self.left {
            out.push_str(body);
            out.extend(std::iter::repeat_n(self.pad, short));
            return;
        }
        let sign = match self.pad == '0' && body.starts_with(['-', '+']) {
            true => 1,
            false => 0,
        };
        out.push_str(&body[..sign]);
        out.extend(std::iter::repeat_n(self.pad, short));
        out.push_str(&body[sign..]);
    }
}

/// One `%s` argument's text — [ADR 0007](../../../../docs/adr/0007-explicit-type-system.md)
/// § 2's rows, reached through the one implementation of them.
///
/// `mwl_runtime::value_to_string` answers a `Tag::Str` carrying exactly one
/// fresh reference, so the handle below owns it and releases it when the
/// borrowed text has been copied out.
fn rendered(argument: &Value) -> Result<String, Fault> {
    let value = mwl_runtime::value_to_string(*argument)?;
    let ptr = value
        .str_ptr()
        .ok_or_else(|| Fault::fatal("`value_to_string` answered something that is not a string"))?;
    #[expect(
        unsafe_code,
        reason = "`value_to_string` hands back exactly one fresh reference, and this handle is \
                  the thing that releases it"
    )]
    let owned = unsafe { MwlStr::from_raw(ptr) };
    std::str::from_utf8(owned.as_bytes())
        .map(str::to_owned)
        .map_err(|_| {
            Fault::fatal(
                "Core\\Str::format() received a `string` that is not valid UTF-8, which ADR 0009 \
                 guarantees it cannot be",
            )
        })
}

/// One argument as the `int` an integer conversion needs.
///
/// A `float` truncates toward zero, as PHP's own `%d` does. A `decimal`
/// follows ADR 0054 § 4 instead: integral and in range, or a throw naming the
/// rounding member, because silent rounding is what that ADR removed.
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncating toward zero is what `%d` over a float means"
)]
fn integer(argument: &Value, conversion: char) -> Result<i64, Fault> {
    match argument.tag() {
        Some(Tag::Int) => argument.as_int().ok_or_else(|| unreadable(conversion)),
        #[expect(
            clippy::cast_possible_wrap,
            reason = "`%u` and the base conversions read the same bit pattern back"
        )]
        Some(Tag::Uint) => argument
            .as_uint()
            .map(|n| n as i64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Bool) => Ok(i64::from(argument.as_bool() == Some(true))),
        Some(Tag::Float) => argument
            .as_float()
            .map(|n| n as i64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Decimal) => argument
            .as_decimal()
            .and_then(mwl_runtime::Decimal::to_i64)
            .ok_or_else(|| {
                Fault::thrown(format!(
                    "Core\\Str::format(): `%{conversion}` needs a whole number, and this \
                     `decimal` is not one — round it with `Core\\Math::floor`, `::ceil` or \
                     `::round` first"
                ))
            }),
        _ => Err(unreadable(conversion)),
    }
}

/// One argument as the `float` a `%f`/`%e`/`%g` needs. A `decimal` converts by
/// ADR 0054 § 4's `decimal → float` row — nearest `f64`, lossy, and asked for
/// explicitly by writing a float conversion.
#[expect(
    clippy::cast_precision_loss,
    reason = "an integer wider than 2^53 loses precision on the way to `%f`, which is what asking \
              for a float conversion means"
)]
fn floating(argument: &Value, conversion: char) -> Result<f64, Fault> {
    match argument.tag() {
        Some(Tag::Float) => argument.as_float().ok_or_else(|| unreadable(conversion)),
        Some(Tag::Int) => argument
            .as_int()
            .map(|n| n as f64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Uint) => argument
            .as_uint()
            .map(|n| n as f64)
            .ok_or_else(|| unreadable(conversion)),
        Some(Tag::Bool) => Ok(f64::from(u8::from(argument.as_bool() == Some(true)))),
        Some(Tag::Decimal) => argument
            .as_decimal()
            .map(mwl_runtime::Decimal::to_f64)
            .ok_or_else(|| unreadable(conversion)),
        _ => Err(unreadable(conversion)),
    }
}

/// `%f`: fixed point, with `precision` digits after the point.
fn fixed(value: f64, precision: usize) -> String {
    match value.is_finite() {
        true => format!("{value:.precision$}"),
        false => nonfinite(value),
    }
}

/// `%e`: one digit, a point, `precision` digits, then `e` and a signed
/// exponent of as few digits as it takes — PHP's shape (`1.234500e+3`), not
/// C's two-digit minimum.
fn scientific(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return nonfinite(value);
    }
    let rendered = format!("{value:.precision$e}");
    // Rust writes the exponent without a `+`; everything else already agrees.
    match rendered.split_once('e') {
        Some((mantissa, exponent)) if !exponent.starts_with('-') => {
            format!("{mantissa}e+{exponent}")
        }
        _ => rendered,
    }
}

/// `%g`: `precision` *significant* digits, rendered as `%f` or `%e` by C's own
/// rule, then with the trailing zeros that rule leaves behind removed.
///
/// The one PHP-specific part is what "removed" means in the `%e` form: PHP
/// keeps a digit after the point (`1.0e+6`, never `1e+6`), while the `%f` form
/// drops the point along with the zeros (`999999`, `0.5`).
fn shortest(value: f64, precision: usize) -> String {
    if !value.is_finite() {
        return nonfinite(value);
    }
    let significant = precision.max(1);
    // The decimal exponent *after* rounding to `significant` digits, read back
    // off the scientific form so that 999999.5 at six digits reports 6.
    let scientific = format!("{value:.*e}", significant - 1);
    let exponent: i32 = scientific
        .split_once('e')
        .and_then(|(_, exponent)| exponent.parse().ok())
        .unwrap_or(0);
    if exponent < -4 || exponent >= i32::try_from(significant).unwrap_or(i32::MAX) {
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let mantissa = match mantissa.contains('.') {
            true => {
                let trimmed = mantissa.trim_end_matches('0');
                match trimmed.ends_with('.') {
                    true => format!("{trimmed}0"),
                    false => trimmed.to_owned(),
                }
            }
            false => format!("{mantissa}.0"),
        };
        let sign = match exponent.starts_with('-') {
            true => "",
            false => "+",
        };
        return format!("{mantissa}e{sign}{exponent}");
    }
    let after = usize::try_from(i32::try_from(significant).unwrap_or(i32::MAX) - 1 - exponent)
        .unwrap_or_default();
    let fixed = format!("{value:.after$}");
    match fixed.contains('.') {
        true => fixed.trim_end_matches('0').trim_end_matches('.').to_owned(),
        false => fixed,
    }
}

/// An infinity or a NaN, in C's spelling — the one shape every `%f`/`%e`/`%g`
/// shares, so the three cannot disagree about it.
fn nonfinite(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    match value.is_sign_negative() {
        true => "-inf".to_owned(),
        false => "inf".to_owned(),
    }
}

/// A placeholder this grammar cannot read at all.
fn malformed(after: &str) -> Fault {
    let shown: String = after.chars().take(8).collect();
    Fault::thrown(format!(
        "Core\\Str::format(): `%{shown}` is not a placeholder this template grammar allows"
    ))
}

/// A value with no reading for the conversion it reached.
fn unreadable(conversion: char) -> Fault {
    Fault::thrown(format!(
        "Core\\Str::format(): `%{conversion}` has no reading for this value"
    ))
}

#[cfg(test)]
mod tests {
    use super::format;
    use mwl_runtime::{Decimal, MwlArray, MwlStr, Value};

    /// One `string` argument, and the reference this test owes for it.
    fn s(text: &str) -> Value {
        Value::str(MwlStr::new(text.as_bytes()))
    }

    /// Releases every reference the arguments of one case carried — `format`
    /// borrows them, so this test still owns them.
    fn released(arguments: Vec<Value>) {
        for argument in arguments {
            #[expect(
                unsafe_code,
                reason = "this test built exactly one reference per argument, and `format` \
                          borrowed rather than consumed it"
            )]
            unsafe {
                argument.release();
            }
        }
    }

    /// Asserts one template's answer and releases its arguments.
    ///
    /// **Every expected string here was produced by PHP 8.5's own `vsprintf`
    /// before it was written down.** This module's whole job is to answer what
    /// those six functions answer, so a hand-derived expectation would be
    /// testing the implementation against itself.
    #[track_caller]
    fn same(template: &str, arguments: Vec<Value>, expected: &str) {
        let answer = format(template, &arguments);
        released(arguments);
        assert_eq!(answer.expect("no failure"), expected, "for `{template}`");
    }

    #[test]
    fn the_conversions_answer_what_php_answers() {
        same("%s has %d", vec![s("fox"), Value::int(2)], "fox has 2");
        same(
            "%s|%s|%s|",
            vec![Value::bool(true), Value::bool(false), Value::null()],
            "1|||",
        );
        same("%d|", vec![Value::float(3.99)], "3|");
        same("%u|", vec![Value::int(-1)], "18446744073709551615|");
        same(
            "%x|%X|%o|%b|",
            vec![
                Value::int(255),
                Value::int(255),
                Value::int(8),
                Value::int(5),
            ],
            "ff|FF|10|101|",
        );
        same("%%|", Vec::new(), "%|");
    }

    #[test]
    fn the_float_conversions_answer_what_php_answers() {
        same(
            "%.3f|%.0f|%f|",
            vec![Value::float(1.23456), Value::float(2.5), Value::float(1.5)],
            "1.235|2|1.500000|",
        );
        same(
            "%e|%e|",
            vec![Value::float(1234.5), Value::float(0.000_123)],
            "1.234500e+3|1.230000e-4|",
        );
        same(
            "%g|%g|%g|",
            vec![
                Value::float(0.0001),
                Value::float(0.000_01),
                Value::float(123_456_789.0),
            ],
            "0.0001|1.0e-5|1.23457e+8|",
        );
        same(
            "%g|%g|%.3g|%.0g|",
            vec![
                Value::float(999_999.0),
                Value::float(1_000_000.0),
                Value::float(123_456.0),
                Value::float(-2.5),
            ],
            "999999|1.0e+6|1.23e+5|-2|",
        );
    }

    #[test]
    fn the_flags_width_and_precision_answer_what_php_answers() {
        same("%5s|%-5s|", vec![s("ab"), s("ab")], "   ab|ab   |");
        same(
            "%05d|%+d|%+d|",
            vec![Value::int(42), Value::int(42), Value::int(-42)],
            "00042|+42|-42|",
        );
        same("%'*8s|", vec![s("hi")], "******hi|");
        same("%'x-8d|", vec![Value::int(42)], "42xxxxxx|");
        same(
            "%10.3f|%-8.2f|",
            vec![Value::float(1.23456), Value::float(1.23456)],
            "     1.235|1.23    |",
        );
        same("%08.3f|", vec![Value::float(-1.23456)], "-001.235|");
        same("%.2s|%5.2s|", vec![s("abcdef"), s("abcdef")], "ab|   ab|");
    }

    /// `%1$s` names an argument rather than consuming the next one, so it can
    /// read the same one twice — PHP's own behaviour, and the reason
    /// "every argument is read" is a rule per argument rather than a count.
    #[test]
    fn a_positional_placeholder_names_its_argument() {
        same("%1$s-%1$s-%2$s", vec![s("a"), s("b")], "a-a-b");
    }

    /// Width and precision measure `Core\Str::length`'s unit, not C's bytes —
    /// this module's own docs own why.
    #[test]
    fn width_measures_what_length_measures() {
        same(
            "%4s|",
            vec![s("\u{1f1e6}\u{1f1f9}")],
            "   \u{1f1e6}\u{1f1f9}|",
        );
        same(
            "%.1s|",
            vec![s("\u{1f1e6}\u{1f1f9}x")],
            "\u{1f1e6}\u{1f1f9}|",
        );
    }

    /// Each refusal this module's docs list, at the boundary that reports it.
    #[test]
    fn every_mismatch_throws_rather_than_producing_something() {
        for (template, arguments) in [
            ("%s %s", vec![s("only one")]),
            ("%s", vec![s("read"), s("unread")]),
            ("%q", Vec::new()),
            ("%", Vec::new()),
            ("%d", vec![Value::array(MwlArray::new())]),
            (
                "%d",
                vec![Value::decimal(Decimal::parse("1.5").expect("a decimal"))],
            ),
        ] {
            let answer = format(template, &arguments);
            released(arguments);
            assert!(
                answer.is_err(),
                "`{template}` produced {:?} rather than throwing",
                answer.ok()
            );
        }
    }

    /// An integral `decimal` reaches `%d` unchanged, and any `decimal` renders
    /// through `%s` at its own scale — ADR 0054 § 4's two rows, side by side.
    #[test]
    fn a_decimal_follows_adr_0054s_conversion_rows() {
        same(
            "%d|%s|",
            vec![
                Value::decimal(Decimal::parse("42").expect("a decimal")),
                Value::decimal(Decimal::parse("19.90").expect("a decimal")),
            ],
            "42|19.90|",
        );
    }
}
