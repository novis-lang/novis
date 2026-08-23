//! Converting a scalar to its `string` form, PHP's way.
//!
//! `int`, `uint` and `bool` are one line each and live at their helpers.
//! `float` is not: PHP's own rule is `zend_gcvt(value, precision, '.', 'E')`
//! with `precision = 14`, which is `%G`-shaped but not `%G` — it strips
//! trailing zeros, keeps one fractional digit in exponential form, and spells
//! the non-finite values `INF`/`-INF`/`NAN`. Rust's `{}` for `f64` does none
//! of those, so the rule is implemented rather than delegated.
//!
//! This is [ADR 0007](../../../docs/adr/0007-explicit-type-system.md)'s
//! conversion table for `float -> string`, which that ADR does not list as a
//! deliberate divergence — so the behaviour has to match PHP's, and it is
//! checked against `php -r` output in this module's tests.
//!
//! `decimal` ([ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md)) is
//! deliberately *not* here: its whole point is a fixed scale with no float
//! round-tripping, so it gets its own formatting when its i128 backend lands.

/// PHP's `precision` ini default, and the significant-digit count
/// `float`-to-`string` conversion rounds to.
const PRECISION: usize = 14;

/// A `float`'s `string` form, exactly as PHP's `echo` would spell it.
///
/// ```
/// use mwl_runtime::php_float_to_string;
///
/// assert_eq!(php_float_to_string(1.0), "1");
/// assert_eq!(php_float_to_string(0.1 + 0.2), "0.3");
/// assert_eq!(php_float_to_string(1e20), "1.0E+20");
/// assert_eq!(php_float_to_string(f64::INFINITY), "INF");
/// ```
#[must_use]
pub fn php_float_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NAN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "INF" } else { "-INF" }.to_owned();
    }

    // Round to PRECISION significant digits first, and read the decimal
    // exponent back off *that* — so a value that carries into the next power
    // of ten while rounding (9.99999999999999e13) picks the same branch PHP
    // does, rather than the branch its unrounded exponent would suggest.
    let scientific = format!("{:.*e}", PRECISION - 1, value);
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("Rust's LowerExp for f64 always emits an exponent");
    let exponent: i32 = exponent
        .parse()
        .expect("Rust's LowerExp for f64 always emits a decimal exponent");

    if exponent < -4 || exponent >= i32::try_from(PRECISION).expect("PRECISION is 14") {
        let mantissa = strip_trailing_zeros(mantissa);
        // PHP keeps one fractional digit in exponential form: `1.0E+20`, not
        // `1E+20`.
        let mantissa = if mantissa.contains('.') {
            mantissa
        } else {
            format!("{mantissa}.0")
        };
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{mantissa}E{sign}{}", exponent.unsigned_abs())
    } else {
        // Fixed form. The digits after the point are whatever is left of the
        // significant-digit budget once the integer part has taken its share.
        let decimals =
            usize::try_from(i32::try_from(PRECISION).expect("PRECISION is 14") - 1 - exponent)
                .expect("exponent is below PRECISION in this branch, so this is non-negative");
        strip_trailing_zeros(&format!("{value:.decimals$}"))
    }
}

/// Drops the trailing zeros of a fractional part, and the point with them if
/// nothing is left after it.
fn strip_trailing_zeros(formatted: &str) -> String {
    if !formatted.contains('.') {
        return formatted.to_owned();
    }
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every expectation here is `php -r 'echo $v;'` output, taken from the
    /// PHP 8.5 build `CLAUDE.md` names as the comparison oracle.
    #[test]
    fn a_float_is_spelled_the_way_php_spells_it() {
        for (value, expected) in [
            (1.0, "1"),
            (0.1 + 0.2, "0.3"),
            (1e20, "1.0E+20"),
            (1.0e-5, "1.0E-5"),
            (0.0, "0"),
            (-0.0, "-0"),
            (1.5, "1.5"),
            (123_456_789_012_345_678.0, "1.2345678901235E+17"),
            (1e14, "1.0E+14"),
            (1e13, "10000000000000"),
            (-2.5e-7, "-2.5E-7"),
            (1.0 / 3.0, "0.33333333333333"),
            (-1.5, "-1.5"),
            (0.0001, "0.0001"),
            (0.000_01, "1.0E-5"),
        ] {
            assert_eq!(php_float_to_string(value), expected, "for {value:?}");
        }
    }

    #[test]
    fn the_non_finite_values_are_spelled_out() {
        assert_eq!(php_float_to_string(f64::NAN), "NAN");
        assert_eq!(php_float_to_string(f64::INFINITY), "INF");
        assert_eq!(php_float_to_string(f64::NEG_INFINITY), "-INF");
    }

    #[test]
    fn stripping_leaves_an_integer_alone() {
        assert_eq!(strip_trailing_zeros("100"), "100");
        assert_eq!(strip_trailing_zeros("1.500"), "1.5");
        assert_eq!(strip_trailing_zeros("1.000"), "1");
        assert_eq!(strip_trailing_zeros("-0.000"), "-0");
    }
}
