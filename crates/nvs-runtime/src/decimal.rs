//! `rule:types/decimal`'s `decimal`
//! scalar: sign, a 96-bit unsigned mantissa and a scale of 0 to 28, with the
//! whole of § 3's arithmetic and § 4's conversions.
//!
//! # Where the sixteen bytes go, and why they are a [`crate::Value`]'s
//!
//! A `decimal` is **exactly a [`crate::Value`] carrying [`Tag::Decimal`]** —
//! not a second 16-byte shape sitting beside one. `rule:types/decimal` asks for a
//! register-pair-sized, allocation-free value, and a `Value` already is one:
//! a tag byte, seven bytes the struct calls padding, and an eight-byte
//! payload. Sign and scale need two of those seven, which leaves twelve bytes
//! — exactly the 96 bits the mantissa needs — spread across the rest of the
//! padding and the payload:
//!
//! ```text
//! bit   0..8    8..16   16..24  24..32   32..128
//!      [tag=10][scale] [sign]  [zero]  [96-bit mantissa]
//! ```
//!
//! That is the little-endian image of the struct, so the same `u128` register
//! pair `nvs_ir::ty::Ty::Tagged` travels in carries a `decimal` with no
//! reshuffling, and [`crate::Value`]'s two-store materialization works
//! unchanged. The alternative — a representation of its own, outside `Value` —
//! would have cost a second widen/narrow protocol and a second helper argument
//! shape, and would have left `mixed` with no way to hold a `decimal` at all.
//! Instead `decimal` inside a `mixed`, a `?decimal` or any other union is the
//! *same bits*: `nvs_ir::InstKind::Tag` and `Untag` are the identity on one.
//!
//! **What it spends:** sixteen bytes per value against eight for a `float`,
//! which is ADR 0054 § *Consequences*' own figure — one extra machine register
//! or eight extra stack bytes per live `decimal`. Nothing is allocated and
//! nothing is refcounted, so there is no per-value heap cost at all.
//!
//! # What throws
//!
//! Every fallible operation here returns `None`, which the helper layer turns
//! into `rule:types/arithmetic`'s `ArithmeticError`. That covers both overflow kinds the
//! ADR names — a mantissa wider than 96 bits *and* a scale that would exceed
//! 28 — plus division by zero, and a conversion whose value does not fit.
//! Division is the one operation that may round: half to even, at the maximum
//! scale the result admits, fixed in the language and not configurable.
//!
//! # Known gaps
//!
//! * **A division whose intermediate exceeds 128 bits throws rather than
//!   rounding.** [`Decimal::checked_div`] folds the two operands' scales into one
//!   numerator or denominator before dividing, and that fold is a
//!   `checked_mul` over `u128`; a quotient that would still have fit is
//!   therefore refused in the corner where both a wide mantissa and a wide
//!   scale difference meet. Closing it wants a 192-bit intermediate, which is
//!   the same wider intermediate ADR 0054 § *Consequences* already predicts.
//!   Decided: Retry at 192 bits only when the 128-bit fold overflows — Exact everywhere, and the common
//!   path keeps today's cost; it adds a second code path.
//!   — owner: unowned-closures
//! * **Nothing inlines.** That ADR expects `+`, `-` and comparison at equal
//!   scale to become i128 instructions in the emitted code; today every
//!   operator is an out-of-line helper call. That is a latency question
//!   (priority 3) to close in the backend, not a semantic one — the results
//!   are identical either way.
//!   — owner: M12

use std::cmp::Ordering;

use crate::value::Tag;

/// `rule:types/decimal`'s mantissa bound: 96 bits, unsigned, with the sign carried
/// beside it rather than in it.
pub const MAX_MANTISSA: u128 = (1u128 << 96) - 1;

/// `rule:types/decimal`'s scale bound: the number of digits after the point.
pub const MAX_SCALE: u8 = 28;

/// `10^n` for every `n` a `u128` can hold, which is every scale this type
/// uses and every intermediate [`Decimal::checked_div`] folds.
const POW10: [u128; 39] = {
    let mut table = [1u128; 39];
    let mut i = 1;
    while i < 39 {
        table[i] = table[i - 1] * 10;
        i += 1;
    }
    table
};

/// `10^n`, or `None` where that does not fit a `u128`.
fn pow10(n: u32) -> Option<u128> {
    POW10.get(usize::try_from(n).ok()?).copied()
}

/// One `decimal` value — see this module's own docs for the layout.
///
/// The value is `(-1)^negative × mantissa × 10^-scale`. Zero is always
/// non-negative: there is no `-0` a program can observe, unlike `float`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Decimal {
    negative: bool,
    scale: u8,
    mantissa: u128,
}

/// What a division threw away when it stopped — the whole of what a rounding
/// mode has to decide against.
///
/// Public because `rule:types/arithmetic` puts the mode itself in `Core\RoundMode`, which
/// is `nvs-stdlib`'s: `Core\Decimal::divRound` reads this and applies the case
/// the caller named. Half is stated as its own answer rather than folded into
/// one of its neighbours precisely because the `Half*` modes exist to part
/// there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Discard {
    /// The division came out even: the quotient is exact at that scale.
    Nothing,
    /// Less than half a unit in the last place produced.
    BelowHalf,
    /// Exactly half — the tie every `Half*` mode is named for.
    Half,
    /// More than half, which every mode but `Down` rounds away from zero.
    AboveHalf,
}

/// The state [`Decimal::long_divide`] stops in, which each of its callers
/// reads its own way.
struct LongDivision {
    negative: bool,
    mantissa: u128,
    scale: u8,
    remainder: u128,
    denominator: u128,
}

impl LongDivision {
    /// Where what is left over sits against half a unit in the last place
    /// produced — the one place that comparison is made.
    fn discarded(&self) -> Discard {
        if self.remainder == 0 {
            return Discard::Nothing;
        }
        match self.remainder.checked_mul(2) {
            // Twice the remainder overflowed where the denominator did not, so
            // it is the larger of the two.
            None => Discard::AboveHalf,
            Some(twice) => match twice.cmp(&self.denominator) {
                Ordering::Less => Discard::BelowHalf,
                Ordering::Equal => Discard::Half,
                Ordering::Greater => Discard::AboveHalf,
            },
        }
    }
}

impl Decimal {
    /// A `decimal` from its three parts, or `None` where either bound is
    /// exceeded.
    #[must_use]
    pub fn new(negative: bool, mantissa: u128, scale: u8) -> Option<Self> {
        if mantissa > MAX_MANTISSA || scale > MAX_SCALE {
            return None;
        }
        Some(Self {
            negative: negative && mantissa != 0,
            scale,
            mantissa,
        })
    }

    /// `0`, at scale zero.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            negative: false,
            scale: 0,
            mantissa: 0,
        }
    }

    /// Whether this is zero, at any scale — what `rule:expressions/truthy-positions`'s truthy table asks.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    /// This value's sign.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.negative
    }

    /// The number of digits after the point — observable, because `rule:types/conversion` makes `19.90 as string` render as `"19.90"`.
    #[must_use]
    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// The unsigned mantissa.
    #[must_use]
    pub const fn mantissa(self) -> u128 {
        self.mantissa
    }

    /// This value's sixteen-byte image, tag byte included — see this module's
    /// own docs for the bit positions, which are the one home for them.
    #[must_use]
    pub fn to_bits(self) -> u128 {
        u128::from(Tag::Decimal as u8)
            | (u128::from(self.scale) << 8)
            | (u128::from(self.negative) << 16)
            | (self.mantissa << 32)
    }

    /// The inverse of [`Self::to_bits`], refusing an image whose tag byte is
    /// not [`Tag::Decimal`] or whose scale is out of range.
    #[must_use]
    pub fn from_bits(bits: u128) -> Option<Self> {
        if u8::try_from(bits & 0xff).ok()? != Tag::Decimal as u8 {
            return None;
        }
        let scale = u8::try_from((bits >> 8) & 0xff).ok()?;
        let negative = (bits >> 16) & 0xff != 0;
        Self::new(negative, bits >> 32, scale)
    }

    /// An `int`, exactly — `rule:types/conversion`'s first row: every `i64` fits in 96
    /// bits.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self {
            negative: value < 0,
            scale: 0,
            mantissa: u128::from(value.unsigned_abs()),
        }
    }

    /// A `uint`, exactly — [`Self::from_i64`]'s row, unsigned.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        Self {
            negative: false,
            scale: 0,
            mantissa: u128::from(value),
        }
    }

    /// A `float`, as `rule:types/conversion` defines that row: "the shortest decimal that
    /// round-trips to that `float`; i.e. exactly the value the float prints
    /// as", so `0.1 as decimal` is `0.1` rather than
    /// `0.1000000000000000055…`. `None` for a NaN, an infinity, or a value
    /// whose shortest form needs more than 96 bits or a scale past 28.
    #[must_use]
    pub fn from_f64(value: f64) -> Option<Self> {
        let (negative, mantissa, scale) = f64_parts(value)?;
        fit(negative, mantissa, scale)
    }

    /// A `string`, as `rule:types/conversion`'s row demands: the **whole** string must be
    /// an exact decimal literal, with no leading-garbage rule.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (negative, digits, scale) = literal_parts(text)?;
        fit(negative, digits, scale)
    }

    /// The nearest `f64` — lossy, and explicit at every call site like every
    /// other `as`.
    ///
    /// Routed through this value's own rendering so the result is the
    /// correctly-rounded nearest `f64` rather than the doubly-rounded one a
    /// `mantissa / 10^scale` division would give.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        self.to_string().parse().unwrap_or(f64::NAN)
    }

    /// Whether this value has no fractional part.
    #[must_use]
    pub fn is_integral(self) -> bool {
        let Some(unit) = pow10(u32::from(self.scale)) else {
            return false;
        };
        self.mantissa.is_multiple_of(unit)
    }

    /// This value as an `int` — integral and in range, or `None`. `rule:types/conversion`: rounding is `Core\Decimal::floor`/`ceil`/`round`, said out loud.
    #[must_use]
    pub fn to_i64(self) -> Option<i64> {
        let whole = self.whole_part()?;
        if !self.negative {
            return i64::try_from(whole).ok();
        }
        match i64::try_from(whole) {
            Ok(magnitude) => Some(-magnitude),
            // `i64::MIN`'s magnitude is one past `i64::MAX`, and is the one
            // value that only exists on the negative side.
            Err(_) => (whole == u128::from(i64::MAX.unsigned_abs()) + 1).then_some(i64::MIN),
        }
    }

    /// This value as a `uint` — [`Self::to_i64`]'s row, unsigned, so a
    /// negative value is refused.
    #[must_use]
    pub fn to_u64(self) -> Option<u64> {
        if self.negative {
            return None;
        }
        u64::try_from(self.whole_part()?).ok()
    }

    /// The integer this value is, or `None` if it has a fractional part.
    fn whole_part(self) -> Option<u128> {
        let unit = pow10(u32::from(self.scale))?;
        self.mantissa
            .is_multiple_of(unit)
            .then(|| self.mantissa / unit)
    }

    /// This value with every trailing zero stripped from its mantissa — the
    /// one form in which two equal decimals written at different scales agree
    /// bit for bit, which is what [`crate::value_hash`] needs and what
    /// [`Self::compare`] answers without needing.
    ///
    /// Never used for rendering: `rule:types/conversion` keeps the scale precisely so
    /// `19.90` renders as `"19.90"`.
    #[must_use]
    pub fn reduced(self) -> Self {
        let mut value = self;
        while value.scale > 0 && value.mantissa.is_multiple_of(10) {
            value.mantissa /= 10;
            value.scale -= 1;
        }
        value
    }

    /// This value's magnitude. Total, unlike `-i64::MIN`: the mantissa is
    /// unsigned, so there is no value whose magnitude does not fit.
    #[must_use]
    pub fn abs(self) -> Self {
        Self {
            negative: false,
            ..self
        }
    }

    /// This value with its sign flipped.
    #[must_use]
    pub fn negated(self) -> Self {
        Self {
            negative: !self.negative && self.mantissa != 0,
            ..self
        }
    }

    /// `a + b` — `rule:types/arithmetic`, throwing rather than wrapping or promoting.
    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        let (scale, a, b) = align(self, other)?;
        if self.negative == other.negative {
            Self::new(self.negative, a.checked_add(b)?, scale)
        } else if a >= b {
            Self::new(self.negative, a - b, scale)
        } else {
            Self::new(other.negative, b - a, scale)
        }
    }

    /// `a - b`.
    #[must_use]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(other.negated())
    }

    /// `a * b`. The scales add, so a product wanting more than 28 of them
    /// throws rather than silently reducing precision the way `System.Decimal`
    /// does — `rule:types/arithmetic`'s deliberate divergence.
    #[must_use]
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let scale = self.scale.checked_add(other.scale)?;
        let mantissa = self.mantissa.checked_mul(other.mantissa)?;
        Self::new(self.negative != other.negative, mantissa, scale)
    }

    /// `a / b` — always a `decimal`, never a union, and the one operation that
    /// may be inexact: it rounds **half to even at the maximum scale the
    /// result admits**, which `rule:types/arithmetic` fixes in the language. A zero
    /// divisor throws.
    #[must_use]
    pub fn checked_div(self, other: Self) -> Option<Self> {
        let division = self.long_divide(other, None)?;
        let mut mantissa = division.mantissa;
        if rounds_away(division.discarded(), mantissa) {
            mantissa = mantissa.checked_add(1)?;
        }
        Self::new(division.negative, mantissa, division.scale)
    }

    /// `a / b` where the quotient is exact — the language-level operator's
    /// division stopped one decision earlier, before the rounding that is the
    /// only inexactness `rule:types/arithmetic` admits.
    ///
    /// `None` covers every refusal `Core\Decimal::divExact` makes: a zero
    /// divisor, a quotient that repeats, and a quotient this type cannot hold.
    /// They are not distinguished because the member does not distinguish
    /// them either — "not exact" and "wider than a `decimal` holds" are one
    /// sentence there, for the reason this module's known gaps state.
    #[must_use]
    pub fn checked_div_exact(self, other: Self) -> Option<Self> {
        let division = self.long_divide(other, None)?;
        if division.remainder != 0 {
            return None;
        }
        Self::new(division.negative, division.mantissa, division.scale)
    }

    /// `a / b` truncated to exactly `scale` fractional digits, with what the
    /// truncation threw away — everything `Core\Decimal::divRound` needs to
    /// apply a rounding mode the caller named, and nothing more.
    ///
    /// Deliberately **not** [`Self::checked_div`] followed by a second
    /// rounding: rounding twice is how a quotient one digit past the target
    /// carries a tie that was never there, and priority 2 does not pay for
    /// that. `None` for a zero divisor, or where the quotient does not fit at
    /// that scale — which a wide enough `scale` always eventually forces.
    #[must_use]
    pub fn checked_div_at_scale(self, other: Self, scale: u8) -> Option<(Self, Discard)> {
        if scale > MAX_SCALE {
            return None;
        }
        let division = self.long_divide(other, Some(scale))?;
        let value = Self::new(division.negative, division.mantissa, division.scale)?;
        Some((value, division.discarded()))
    }

    /// The one long division under [`Self::checked_div`],
    /// [`Self::checked_div_exact`] and [`Self::checked_div_at_scale`], which
    /// differ only in what they do with the state it stops in.
    ///
    /// `limit` is `None` for the adaptive scale the operator answers at — grow
    /// until the remainder is gone, the scale bound is reached or the mantissa
    /// would not hold another digit — and `Some(scale)` for exactly that many
    /// fractional digits, where running out of mantissa is a refusal rather
    /// than a shorter answer.
    fn long_divide(self, other: Self, limit: Option<u8>) -> Option<LongDivision> {
        if other.mantissa == 0 {
            return None;
        }
        let negative = self.negative != other.negative;
        if self.mantissa == 0 {
            // Answered before the fold below, which can overflow on a divisor
            // this one does not need to look at.
            return Some(LongDivision {
                negative,
                mantissa: 0,
                scale: limit.unwrap_or(0),
                remainder: 0,
                denominator: other.mantissa,
            });
        }
        // Fold both scales into one side, so what is left is the plain
        // rational `numerator / denominator` and the quotient's scale is
        // exactly the number of fractional digits produced below.
        let (mut numerator, mut denominator) = (self.mantissa, other.mantissa);
        match i32::from(other.scale) - i32::from(self.scale) {
            shift if shift > 0 => {
                numerator = numerator.checked_mul(pow10(shift.unsigned_abs())?)?;
            }
            shift if shift < 0 => {
                denominator = denominator.checked_mul(pow10(shift.unsigned_abs())?)?;
            }
            _ => {}
        }
        let mut mantissa = numerator / denominator;
        let mut remainder = numerator % denominator;
        if mantissa > MAX_MANTISSA {
            return None;
        }
        // A fixed scale keeps producing digits after the remainder is gone —
        // that is what pads `0.25` out to `0.2500` — while the adaptive one
        // stops the moment there is nothing left to divide. Where the mantissa
        // runs out, the adaptive division answers at the scale it reached and
        // the fixed one refuses, because it was asked for a scale it cannot
        // hold rather than for as much as fits.
        let fixed = limit.is_some();
        let stop = limit.unwrap_or(MAX_SCALE);
        let mut scale = 0u8;
        while scale < stop && (fixed || remainder != 0) {
            let (Some(shifted), Some(carried)) =
                (mantissa.checked_mul(10), remainder.checked_mul(10))
            else {
                if fixed {
                    return None;
                }
                break;
            };
            let next = shifted + carried / denominator;
            if next > MAX_MANTISSA {
                if fixed {
                    return None;
                }
                break;
            }
            mantissa = next;
            remainder = carried % denominator;
            scale += 1;
        }
        Some(LongDivision {
            negative,
            mantissa,
            scale,
            remainder,
            denominator,
        })
    }

    /// `a % b` — the remainder at the wider of the two scales, carrying the
    /// dividend's sign the way PHP's `%` does.
    #[must_use]
    pub fn checked_rem(self, other: Self) -> Option<Self> {
        if other.mantissa == 0 {
            return None;
        }
        let (scale, a, b) = align(self, other)?;
        Self::new(self.negative, a % b, scale)
    }

    /// Exact ordering against another `decimal`, over the full range of both
    /// and independent of scale: `1.10` and `1.1000` are equal.
    #[must_use]
    pub fn compare(self, other: Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_magnitude(
                self.mantissa,
                i32::from(self.scale),
                other.mantissa,
                i32::from(other.scale),
            ),
            (true, true) => cmp_magnitude(
                other.mantissa,
                i32::from(other.scale),
                self.mantissa,
                i32::from(self.scale),
            ),
        }
    }

    /// Exact ordering against a `float`, which `rule:types/arithmetic` permits where
    /// `decimal ⊕ float` *arithmetic* is a compile error — an exact comparison
    /// is always computable even where a common arithmetic type is not.
    ///
    /// `None` for a NaN, the one `f64` that orders against nothing. The `f64`
    /// is read at the value it prints as, which is the same reading `rule:types/conversion`'s `float → decimal` row takes, so `0.1 as decimal == 0.1` holds.
    #[must_use]
    pub fn compare_f64(self, other: f64) -> Option<Ordering> {
        if other.is_nan() {
            return None;
        }
        if other.is_infinite() {
            return Some(if other > 0.0 {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let (negative, mantissa, scale) = f64_parts(other)?;
        let negative = negative && mantissa != 0;
        Some(match (self.negative, negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => cmp_magnitude(self.mantissa, i32::from(self.scale), mantissa, scale),
            (true, true) => cmp_magnitude(mantissa, scale, self.mantissa, i32::from(self.scale)),
        })
    }
}

/// `rule:types/conversion`'s `decimal → string` row: total, and **scale-preserving**, so
/// `19.90` renders as `"19.90"` rather than losing the trailing zero the way
/// every PHP application re-derives with `number_format`.
impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            f.write_str("-")?;
        }
        let digits = self.mantissa.to_string();
        let scale = usize::from(self.scale);
        if scale == 0 {
            return f.write_str(&digits);
        }
        let padded = if digits.len() <= scale {
            format!("{}{digits}", "0".repeat(scale - digits.len() + 1))
        } else {
            digits
        };
        let split = padded.len() - scale;
        write!(f, "{}.{}", &padded[..split], &padded[split..])
    }
}

/// Both mantissas at the wider of the two scales, or `None` where widening
/// the narrower one overflows 96 bits.
fn align(a: Decimal, b: Decimal) -> Option<(u8, u128, u128)> {
    let scale = a.scale.max(b.scale);
    let widen = |value: Decimal| {
        let factor = pow10(u32::from(scale - value.scale))?;
        let widened = value.mantissa.checked_mul(factor)?;
        (widened <= MAX_MANTISSA).then_some(widened)
    };
    Some((scale, widen(a)?, widen(b)?))
}

/// Half-to-even, asked of a division that discarded `discard` having produced
/// `mantissa`: round away from zero when what was thrown away is more than
/// half, and on exactly half only when that makes the last digit even.
///
/// The operator's mode, and the only one fixed in the language — `rule:types/arithmetic`.
/// `Core\Decimal::divRound`'s other modes read the same [`Discard`] and part
/// from this one only at the tie.
fn rounds_away(discard: Discard, mantissa: u128) -> bool {
    match discard {
        Discard::Nothing | Discard::BelowHalf => false,
        Discard::Half => mantissa % 2 == 1,
        Discard::AboveHalf => true,
    }
}

/// Compares `a × 10^-ka` against `b × 10^-kb`, both non-negative, exactly and
/// at any scale either side names — including the wide negative scales a
/// `float`'s shortest form produces.
fn cmp_magnitude(a: u128, ka: i32, b: u128, kb: i32) -> Ordering {
    if a == 0 || b == 0 {
        // With one side zero the scales say nothing: a non-zero mantissa at
        // any scale is larger, and two zeros are equal.
        return a.cmp(&b);
    }
    match ka.cmp(&kb) {
        Ordering::Equal => a.cmp(&b),
        // `a` carries more fractional digits, so `b` is the one to widen.
        Ordering::Greater => match u32::try_from(ka - kb).ok().and_then(pow10) {
            Some(factor) => match b.checked_mul(factor) {
                Some(widened) => a.cmp(&widened),
                None => Ordering::Less,
            },
            None => Ordering::Less,
        },
        Ordering::Less => match u32::try_from(kb - ka).ok().and_then(pow10) {
            Some(factor) => match a.checked_mul(factor) {
                Some(widened) => widened.cmp(&b),
                None => Ordering::Greater,
            },
            None => Ordering::Greater,
        },
    }
}

/// A finite `f64` as `(negative, mantissa, scale)` with
/// `value = (-1)^negative × mantissa × 10^-scale`, read from Rust's own
/// shortest-round-trip rendering — which is exactly `rule:types/conversion`'s definition
/// of what a `float`'s decimal value *is*.
///
/// The scale is an `i32` rather than a `u8` because this is also the input to
/// [`cmp_magnitude`], which orders values far outside `decimal`'s own range.
fn f64_parts(value: f64) -> Option<(bool, u128, i32)> {
    if !value.is_finite() {
        return None;
    }
    literal_parts(&format!("{value:e}"))
}

/// Places a parsed literal's parts into `decimal`'s own range: a negative
/// scale is an integer whose point has moved right, so the mantissa absorbs
/// it, and everything else is [`Decimal::new`]'s two bounds.
fn fit(negative: bool, mantissa: u128, scale: i32) -> Option<Decimal> {
    if scale < 0 {
        let widened = mantissa.checked_mul(pow10(scale.unsigned_abs())?)?;
        return Decimal::new(negative, widened, 0);
    }
    Decimal::new(negative, mantissa, u8::try_from(scale).ok()?)
}

/// Splits an exact decimal literal — sign, digits, optional fraction,
/// optional exponent — into `(negative, mantissa, scale)`, with the exponent
/// folded into the scale. `None` for anything that is not one *in full*: ADR
/// 0054 § 4 gives `string → decimal` no leading-garbage rule, exactly as
/// `string → int` has none.
///
/// The scale is an `i32` and may be **negative**, which is a value whose point
/// has moved right of its last digit — `1e308` is mantissa 1 at scale -308.
/// [`fit`] is what narrows that to `decimal`'s own range; [`cmp_magnitude`]
/// deliberately does not, so a comparison against a `float` far outside that
/// range stays exact.
///
/// Trailing zeros are kept, since scale is observable in rendering: `19.90` is
/// a scale-2 value and not a second spelling of `19.9`.
fn literal_parts(text: &str) -> Option<(bool, u128, i32)> {
    let (negative, rest) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (numeric, exponent) = match rest.split_once(['e', 'E']) {
        Some((numeric, exponent)) => (numeric, exponent.parse::<i32>().ok()?),
        None => (rest, 0),
    };
    let (whole, fraction) = numeric.split_once('.').unwrap_or((numeric, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole
        .bytes()
        .chain(fraction.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    let mut digits = String::with_capacity(whole.len() + fraction.len());
    digits.push_str(whole);
    digits.push_str(fraction);
    Some((negative, digits.parse::<u128>().ok()?, scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(text: &str) -> Decimal {
        Decimal::parse(text).expect("a literal this test wrote is in range")
    }

    #[test]
    fn a_decimal_is_a_value_sized_image_with_the_tag_first() {
        let d = dec("19.99");
        let bits = d.to_bits();
        assert_eq!(u8::try_from(bits & 0xff), Ok(Tag::Decimal as u8));
        assert_eq!(Decimal::from_bits(bits), Some(d));
        assert_eq!(d.mantissa(), 1999);
        assert_eq!(d.scale(), 2);
    }

    #[test]
    fn an_image_whose_tag_is_not_decimal_decodes_to_nothing() {
        assert_eq!(Decimal::from_bits(0), None);
        assert_eq!(Decimal::from_bits(u128::from(Tag::Int as u8)), None);
    }

    #[test]
    fn the_full_mantissa_and_scale_range_round_trips() {
        let widest = Decimal::new(true, MAX_MANTISSA, MAX_SCALE).expect("at the bound");
        assert_eq!(Decimal::from_bits(widest.to_bits()), Some(widest));
        assert_eq!(Decimal::new(false, MAX_MANTISSA + 1, 0), None);
        assert_eq!(Decimal::new(false, 1, MAX_SCALE + 1), None);
    }

    #[test]
    fn scale_survives_rendering() {
        assert_eq!(dec("19.90").to_string(), "19.90");
        assert_eq!(dec("0.005").to_string(), "0.005");
        assert_eq!(dec("-0.5").to_string(), "-0.5");
        assert_eq!(dec("42").to_string(), "42");
        assert_eq!(Decimal::new(false, 0, 2).expect("zero").to_string(), "0.00");
    }

    #[test]
    fn the_money_case_is_exact() {
        assert_eq!(dec("0.1").checked_add(dec("0.2")), Some(dec("0.3")));
        assert_eq!(dec("19.99").checked_add(dec("4.95")), Some(dec("24.94")));
        assert_eq!(dec("1.00").checked_sub(dec("0.99")), Some(dec("0.01")));
    }

    #[test]
    fn equality_ignores_scale_but_rendering_does_not() {
        assert_eq!(dec("1.10").compare(dec("1.1000")), Ordering::Equal);
        assert_ne!(dec("1.10").to_string(), dec("1.1000").to_string());
    }

    #[test]
    fn a_sign_change_crossing_zero_lands_on_the_right_side() {
        assert_eq!(dec("1.5").checked_sub(dec("2.0")), Some(dec("-0.5")));
        assert_eq!(dec("-1.5").checked_add(dec("2.0")), Some(dec("0.5")));
        assert!(
            dec("0.0")
                .checked_sub(dec("0.0"))
                .is_some_and(|d| !d.is_negative())
        );
    }

    #[test]
    fn multiplication_adds_the_scales_and_refuses_a_wider_product() {
        assert_eq!(dec("2.50").checked_mul(dec("4.00")), Some(dec("10.0000")));
        let deep = Decimal::new(false, 1, 20).expect("scale 20");
        assert_eq!(deep.checked_mul(deep), None);
        let wide = Decimal::new(false, MAX_MANTISSA, 0).expect("at the bound");
        assert_eq!(wide.checked_mul(dec("10")), None);
    }

    #[test]
    fn division_rounds_half_to_even_at_the_widest_scale_it_admits() {
        let third = dec("1.00").checked_div(dec("3")).expect("a quotient");
        assert_eq!(third.scale(), MAX_SCALE);
        assert_eq!(third.to_string(), "0.3333333333333333333333333333");
        assert_eq!(dec("10").checked_div(dec("4")), Some(dec("2.5")));
        assert_eq!(dec("1").checked_div(dec("8")), Some(dec("0.125")));
        // Exactly half at the last admissible digit, twice: up to the even
        // neighbour once, down to it the other time.
        assert_eq!(dec("0.5").checked_div(dec("2")), Some(dec("0.25")));
        assert_eq!(dec("0").checked_div(dec("3")), Some(Decimal::zero()));
        assert_eq!(dec("1").checked_div(dec("0")), None);
    }

    #[test]
    fn a_remainder_keeps_the_dividends_sign() {
        assert_eq!(dec("7.5").checked_rem(dec("2")), Some(dec("1.5")));
        assert_eq!(dec("-7.5").checked_rem(dec("2")), Some(dec("-1.5")));
        assert_eq!(dec("1").checked_rem(dec("0")), None);
    }

    #[test]
    fn a_float_is_read_at_the_value_it_prints_as() {
        assert_eq!(Decimal::from_f64(0.1), Some(dec("0.1")));
        assert_eq!(Decimal::from_f64(19.99), Some(dec("19.99")));
        assert_eq!(Decimal::from_f64(1e30), None);
        assert_eq!(Decimal::from_f64(f64::NAN), None);
        assert_eq!(Decimal::from_f64(f64::INFINITY), None);
        assert_eq!(dec("19.99").to_f64(), 19.99);
        assert_eq!(dec("-0.125").to_f64(), -0.125);
    }

    #[test]
    fn a_comparison_against_a_float_is_exact_at_any_magnitude() {
        assert_eq!(dec("1.5").compare_f64(1.5), Some(Ordering::Equal));
        assert_eq!(dec("1.5").compare_f64(1.6), Some(Ordering::Less));
        assert_eq!(dec("0").compare_f64(5e-324), Some(Ordering::Less));
        assert_eq!(dec("0").compare_f64(-5e-324), Some(Ordering::Greater));
        assert_eq!(dec("1").compare_f64(1e308), Some(Ordering::Less));
        assert_eq!(dec("1").compare_f64(f64::INFINITY), Some(Ordering::Less));
        assert_eq!(
            dec("1").compare_f64(f64::NEG_INFINITY),
            Some(Ordering::Greater)
        );
        assert_eq!(dec("1").compare_f64(f64::NAN), None);
    }

    #[test]
    fn parsing_takes_the_whole_string_or_nothing() {
        assert_eq!(Decimal::parse("12abc"), None);
        assert_eq!(Decimal::parse("abc"), None);
        assert_eq!(Decimal::parse(""), None);
        assert_eq!(Decimal::parse(" 1"), None);
        assert_eq!(Decimal::parse("1.5e3"), Some(dec("1500")));
        assert_eq!(
            Decimal::parse("-0.0"),
            Some(Decimal::new(false, 0, 1).expect("zero"))
        );
        assert_eq!(Decimal::parse("1e-30"), None);
    }

    #[test]
    fn an_integer_round_trips_and_a_fraction_refuses_to() {
        assert_eq!(Decimal::from_i64(i64::MIN).to_i64(), Some(i64::MIN));
        assert_eq!(Decimal::from_i64(-7).to_i64(), Some(-7));
        assert_eq!(Decimal::from_u64(u64::MAX).to_u64(), Some(u64::MAX));
        assert_eq!(dec("1.5").to_i64(), None);
        assert_eq!(dec("-1").to_u64(), None);
        assert_eq!(dec("2.00").to_i64(), Some(2));
    }

    #[test]
    fn zero_is_never_negative() {
        assert!(!dec("-0.00").is_negative());
        assert!(dec("-0.00").is_zero());
        assert_eq!(dec("-0.00").compare(Decimal::zero()), Ordering::Equal);
    }
}
