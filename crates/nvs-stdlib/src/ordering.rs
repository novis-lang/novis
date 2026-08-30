//! The natural ordering of two `Value`s — what "smaller" means when no
//! comparator was given.
//!
//! Not a domain: [`crate::arr`]'s `sort`/`min`/`max` and [`crate::math`]'s
//! `min`/`max`/`clamp` are the same question asked about a container and about
//! two loose values, and a domain that decided it for itself would make
//! `Math::min($a, $b)` and `Arr::min([$a, $b])` answerable differently. It
//! lives beside [`crate::granularity`] for exactly the reason that module
//! states: more than one domain reaches for it, so no domain owns it.

use nvs_runtime::{Decimal, Fault, Tag, Value};

/// The natural ordering of two values, or a throw for a pair that has none.
///
/// One row per representation, and nothing crosses between rows except the
/// numeric ones:
///
/// * `null` — one value, so always equal.
/// * `bool` — `false` before `true`.
/// * `int`/`uint` — exactly, through `i128`, so no large `uint` is rounded.
/// * `decimal` against anything numeric — exactly, through
///   [`Decimal::compare`] and [`Decimal::compare_f64`]. It orders against the
///   other numeric rows rather than only against its own, because
///   [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md) § 4's first row
///   makes an `int`/`uint` exact as a `decimal` and its § 3 permits the
///   `decimal`/`float` comparison even where their *arithmetic* has no common
///   type. Scale does not enter it: `1.10` and `1.1000` are equal.
/// * `float` against anything numeric — `f64::total_cmp`, which is a real
///   total order (unlike `partial_cmp`, which a `NaN` makes intransitive and
///   therefore unusable by any sort at all). Its two visible consequences are
///   that `-0.0` sorts before `0.0` and that `NaN` sorts at one end rather
///   than throwing.
/// * `string`/`bytes` — **bytewise**, never numerically. See
///   [`crate::arr::nvs_core_arr_sort`], which owns that divergence from PHP.
///
/// Anything else — an object, an array, or two different rows above — is
/// `THROWN`, naming both tags and the `member` that asked. An object is the
/// one worth calling out: ADR 0013 makes `Comparable` the answer, and reaching
/// an instance method from a helper is the thing that is not built yet.
pub(crate) fn compare_values(
    left: &Value,
    right: &Value,
    member: &str,
) -> Result<std::cmp::Ordering, Fault> {
    use std::cmp::Ordering;

    if let (Some(Tag::Null), Some(Tag::Null)) = (left.tag(), right.tag()) {
        return Ok(Ordering::Equal);
    }
    if let (Some(a), Some(b)) = (left.as_bool(), right.as_bool()) {
        return Ok(a.cmp(&b));
    }
    if let (Some(a), Some(b)) = (left.as_str_bytes(), right.as_str_bytes()) {
        return Ok(a.cmp(b));
    }
    // Ahead of the block below rather than inside it: a `decimal` is exact and
    // the widening that would put it into `Numeric` is not, so the pair is
    // answered where both sides are still what they are.
    if let Some(a) = left.as_decimal() {
        if let Some(order) = against_decimal(a, right) {
            return Ok(order);
        }
    } else if let Some(b) = right.as_decimal()
        && let Some(order) = against_decimal(b, left)
    {
        return Ok(order.reverse());
    }
    if let (Some(a), Some(b)) = (numeric(left), numeric(right)) {
        return Ok(match (a, b) {
            (Numeric::Integer(a), Numeric::Integer(b)) => a.cmp(&b),
            (Numeric::Integer(a), Numeric::Real(b)) => real(a).total_cmp(&b),
            (Numeric::Real(a), Numeric::Integer(b)) => a.total_cmp(&real(b)),
            (Numeric::Real(a), Numeric::Real(b)) => a.total_cmp(&b),
        });
    }
    Err(Fault::thrown(format!(
        "{member} has no natural order for tag {} against tag {}: two numbers, two strings, two \
         bools or two nulls have one and no other pair does. `Core\\Arr::sort` takes \
         `{{comparator: ...}}`; for an object ADR 0013 makes `Comparable` the answer",
        left.tag_byte(),
        right.tag_byte()
    )))
}

/// A comparator's verdict as an [`std::cmp::Ordering`] — negative, zero or
/// positive, exactly `usort`'s contract.
///
/// A `float` verdict is accepted for the same reason `int` is: the contract is
/// about the *sign*, and a comparator written as a subtraction of two floats
/// is the shape PHP code already has. A `NaN` has no sign, so it is a throw
/// rather than a silent `Equal`.
///
/// Here rather than on [`crate::arr`] for this module's own reason: a heap
/// takes a comparator too ([`crate::heap`]), and two domains reading one
/// verdict differently is exactly what a shared home prevents. `member` is the
/// fully-qualified member, since the two spell theirs differently.
pub(crate) fn comparator_sign(verdict: Value, member: &str) -> Result<std::cmp::Ordering, Fault> {
    if let Some(int) = verdict.as_int() {
        return Ok(int.cmp(&0));
    }
    if let Some(uint) = verdict.as_uint() {
        return Ok(uint.cmp(&0));
    }
    if let Some(float) = verdict.as_float() {
        return float.partial_cmp(&0.0).ok_or_else(|| {
            Fault::thrown(format!(
                "{member}'s comparator returned NaN, which has no ordering"
            ))
        });
    }
    Err(Fault::fatal(format!(
        "{member}'s comparator returned tag {}, not a number",
        verdict.tag_byte()
    )))
}

/// A `decimal` against any other number, or `None` for a value that is not
/// one — in which case the pair has no numeric order and
/// [`compare_values`]'s throw is the answer.
///
/// Every row is exact: ADR 0054 § 4 makes an `int`/`uint` exact as a
/// `decimal`, and [`Decimal::compare_f64`] reads the `float` at the value it
/// prints as rather than widening either side.
fn against_decimal(left: Decimal, right: &Value) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;

    if let Some(other) = right.as_decimal() {
        return Some(left.compare(other));
    }
    if let Some(int) = right.as_int() {
        return Some(left.compare(Decimal::from_i64(int)));
    }
    if let Some(uint) = right.as_uint() {
        return Some(left.compare(Decimal::from_u64(uint)));
    }
    let float = right.as_float()?;
    // The `float` row's own reading of a `NaN`, so a sort over a mixed array
    // stays total whichever pair it happens to ask about: `f64::total_cmp`
    // puts a `NaN` at one end by its sign bit, and this puts the `decimal` on
    // the other side of it.
    Some(left.compare_f64(float).unwrap_or({
        if float.is_sign_negative() {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }))
}

/// One value's numeric content, or `None` for a value that has none.
#[derive(Clone, Copy)]
enum Numeric {
    /// An `int` or a `uint`, widened so the two compare exactly.
    Integer(i128),
    /// A `float`.
    Real(f64),
}

fn numeric(value: &Value) -> Option<Numeric> {
    if let Some(int) = value.as_int() {
        return Some(Numeric::Integer(i128::from(int)));
    }
    if let Some(uint) = value.as_uint() {
        return Some(Numeric::Integer(i128::from(uint)));
    }
    value.as_float().map(Numeric::Real)
}

/// An exact integer as the `f64` it is compared against.
#[expect(
    clippy::cast_precision_loss,
    reason = "an integer past 2^53 loses low bits on the way to `f64`, which \
              is the same rounding ADR 0007 § 4's int-to-float widening \
              already allows; the alternative is a mixed int/float array \
              having no order at all"
)]
fn real(value: i128) -> f64 {
    value as f64
}
