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

impl Discard {
    /// Where what a division left over sits against half a unit in the last
    /// place produced — the one place that comparison is made.
    fn of(remainder: u128, denominator: u128) -> Self {
        if remainder == 0 {
            return Self::Nothing;
        }
        match remainder.checked_mul(2) {
            // Twice the remainder overflowed where the denominator did not, so
            // it is the larger of the two.
            None => Self::AboveHalf,
            Some(twice) => Self::at(twice.cmp(&denominator)),
        }
    }

    /// The same comparison at [`U192`], for the division that took the wider
    /// intermediate.
    fn of_wide(remainder: U192, denominator: U192) -> Self {
        if remainder.is_zero() {
            return Self::Nothing;
        }
        match remainder.checked_double() {
            None => Self::AboveHalf,
            Some(twice) => Self::at(twice.cmp(&denominator)),
        }
    }

    /// Twice the remainder placed against the denominator, which is the whole
    /// of what either width has to say.
    fn at(order: Ordering) -> Self {
        match order {
            Ordering::Less => Self::BelowHalf,
            Ordering::Equal => Self::Half,
            Ordering::Greater => Self::AboveHalf,
        }
    }
}

/// The state [`Decimal::long_divide`] stops in, which each of its callers
/// reads its own way. What was thrown away is carried as the answer a rounding
/// mode reads rather than as the pair it was computed from, so the two widths
/// the division runs at meet here and nowhere later.
struct LongDivision {
    negative: bool,
    mantissa: u128,
    scale: u8,
    discard: Discard,
}

/// A 192-bit unsigned integer, held as two `u128` halves of which the upper
/// one carries 64 significant bits.
///
/// It exists for [`Decimal::long_divide_at_192`] and nothing else, so it
/// carries exactly the operations that division performs and no more. 192 bits
/// is the width at which every `decimal` division is exact: the widest
/// intermediate any of them folds is a 96-bit mantissa against a scale
/// difference of at most 28, which is `(2^96 - 1) × 10^28` and under 2^190.
///
/// `hi` is a `u128` rather than the `u64` it otherwise is because
/// [`Self::shifted_left`] lets it reach one bit past 64 in the middle of a
/// division step, before the subtraction that brings it back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct U192 {
    hi: u128,
    lo: u128,
}

impl U192 {
    const ZERO: Self = Self { hi: 0, lo: 0 };

    /// The 64 bits each half is read in.
    const LOW_MASK: u128 = (1u128 << 64) - 1;

    const fn from_u128(value: u128) -> Self {
        Self { hi: 0, lo: value }
    }

    /// The value where it fits 128 bits — which every quotient a `decimal` can
    /// hold does, and not every quotient this width can carry.
    const fn to_u128(self) -> Option<u128> {
        if self.hi == 0 { Some(self.lo) } else { None }
    }

    const fn is_zero(self) -> bool {
        self.hi == 0 && self.lo == 0
    }

    /// `self × 10^n`, or `None` past 192 bits.
    fn checked_mul_pow10(self, n: u32) -> Option<Self> {
        (0..n).try_fold(self, |value, _| value.checked_mul_10())
    }

    /// `self × 10`, or `None` past 192 bits.
    fn checked_mul_10(self) -> Option<Self> {
        let low = (self.lo & Self::LOW_MASK) * 10;
        let high = (self.lo >> 64) * 10 + (low >> 64);
        let hi = self.hi * 10 + (high >> 64);
        (hi <= Self::LOW_MASK).then_some(Self {
            hi,
            lo: ((high & Self::LOW_MASK) << 64) | (low & Self::LOW_MASK),
        })
    }

    /// `self × 2`, or `None` past 192 bits.
    fn checked_double(self) -> Option<Self> {
        let doubled = self.shifted_left();
        (doubled.hi <= Self::LOW_MASK).then_some(doubled)
    }

    /// `self × 2` with the upper half left free to carry the bit that takes it
    /// past 64, which [`Self::div_rem`] leans on and no value outlives.
    fn shifted_left(self) -> Self {
        Self {
            hi: (self.hi << 1) | (self.lo >> 127),
            lo: self.lo << 1,
        }
    }

    /// `self - other`, where the caller has already compared the two.
    fn wrapping_sub(self, other: Self) -> Self {
        let (lo, borrow) = self.lo.overflowing_sub(other.lo);
        Self {
            hi: self
                .hi
                .wrapping_sub(other.hi)
                .wrapping_sub(u128::from(borrow)),
            lo,
        }
    }

    fn bit(self, index: u32) -> bool {
        if index < 128 {
            (self.lo >> index) & 1 == 1
        } else {
            (self.hi >> (index - 128)) & 1 == 1
        }
    }

    fn set_bit(&mut self, index: u32) {
        if index < 128 {
            self.lo |= 1 << index;
        } else {
            self.hi |= 1 << (index - 128);
        }
    }

    /// `(self / divisor, self % divisor)` by shift and subtract, one bit at a
    /// time, for a divisor every call site has already found non-zero.
    ///
    /// A machine has no 192-bit divide to reach for, and this path is entered
    /// only by the division whose 128-bit fold overflowed, so the bit loop
    /// buys exactness at a cost the common path never sees.
    fn div_rem(self, divisor: Self) -> (Self, Self) {
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;
        for index in (0..192u32).rev() {
            remainder = remainder.shifted_left();
            if self.bit(index) {
                remainder.lo |= 1;
            }
            if remainder >= divisor {
                remainder = remainder.wrapping_sub(divisor);
                quotient.set_bit(index);
            }
        }
        (quotient, remainder)
    }
}

impl Ord for U192 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hi.cmp(&other.hi).then_with(|| self.lo.cmp(&other.lo))
    }
}

impl PartialOrd for U192 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
        Self::read(text).ok()
    }

    /// [`Self::parse`], with its two failures told apart for a caller that
    /// must act differently on each.
    ///
    /// A database driver is that caller. A column holding a number past this
    /// type's range is a schema an operator narrows — the column's own type,
    /// or a cast in the statement — where a rendering that is not a decimal
    /// literal at all is the driver reading a body it was not sent. `None`
    /// says neither, and one message for both sends the operator looking for
    /// corruption that is not there.
    ///
    /// # Errors
    ///
    /// [`NotDecimal::Unreadable`] for text that is not an exact decimal
    /// literal in full, and [`NotDecimal::PastRange`] for one that is, whose
    /// value needs more than 96 mantissa bits or a scale past 28.
    pub fn read(text: &str) -> Result<Self, NotDecimal> {
        let (negative, digits, scale) = read_parts(text)?;
        fit(negative, digits, scale).ok_or(NotDecimal::PastRange)
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
        if rounds_away(division.discard, mantissa) {
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
        if division.discard != Discard::Nothing {
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
        Some((value, division.discard))
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
                discard: Discard::Nothing,
            });
        }
        // Fold both scales into one side, so what is left is the plain
        // rational `numerator / denominator` and the quotient's scale is
        // exactly the number of fractional digits produced below.
        let shift = i32::from(other.scale) - i32::from(self.scale);
        let factor = pow10(shift.unsigned_abs())?;
        let folded = match shift.cmp(&0) {
            Ordering::Greater => self
                .mantissa
                .checked_mul(factor)
                .map(|n| (n, other.mantissa)),
            Ordering::Less => other
                .mantissa
                .checked_mul(factor)
                .map(|d| (self.mantissa, d)),
            Ordering::Equal => Some((self.mantissa, other.mantissa)),
        };
        let Some((numerator, denominator)) = folded else {
            // The fold is the only step of this division that refuses a
            // quotient the type could have held, so it is the only one the
            // wider intermediate is ever paid for.
            return self.long_divide_at_192(other, limit, negative, shift);
        };
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
            discard: Discard::of(remainder, denominator),
        })
    }

    /// [`Self::long_divide`] over again at 192 bits, entered only where the
    /// 128-bit scale fold overflowed — the wider intermediate ADR 0054
    /// § *Consequences* predicts, and the reason a quotient that fits 96 bits
    /// is never refused for want of room to reach it.
    ///
    /// Every quantity here is bounded by that fold, `(2^96 - 1) × 10^28`,
    /// which is under 2^190: the digit loop only ever multiplies a remainder
    /// already smaller than the denominator, and a mantissa already known to
    /// fit 96 bits. The two paths answer the same division and part only in
    /// the width they reach it at, so a `decimal` is exact over the whole of
    /// its range and rounds in one place, `rule:types/arithmetic`'s half to even.
    ///
    /// **What it spends:** nothing per request and nothing per value — the
    /// wider intermediate is two `u128` temporaries in one stack frame, and
    /// the division whose fold fit never builds one.
    fn long_divide_at_192(
        self,
        other: Self,
        limit: Option<u8>,
        negative: bool,
        shift: i32,
    ) -> Option<LongDivision> {
        let widen =
            |mantissa: u128| U192::from_u128(mantissa).checked_mul_pow10(shift.unsigned_abs());
        let (numerator, denominator) = if shift > 0 {
            (widen(self.mantissa)?, U192::from_u128(other.mantissa))
        } else {
            (U192::from_u128(self.mantissa), widen(other.mantissa)?)
        };
        let (quotient, mut remainder) = numerator.div_rem(denominator);
        let mut mantissa = quotient.to_u128()?;
        if mantissa > MAX_MANTISSA {
            return None;
        }
        let fixed = limit.is_some();
        let stop = limit.unwrap_or(MAX_SCALE);
        let mut scale = 0u8;
        while scale < stop && (fixed || !remainder.is_zero()) {
            let digit = remainder.checked_mul_10().and_then(|carried| {
                let (digit, rest) = carried.div_rem(denominator);
                let next = mantissa.checked_mul(10)?.checked_add(digit.to_u128()?)?;
                (next <= MAX_MANTISSA).then_some((next, rest))
            });
            let Some((next, rest)) = digit else {
                if fixed {
                    return None;
                }
                break;
            };
            mantissa = next;
            remainder = rest;
            scale += 1;
        }
        Some(LongDivision {
            negative,
            mantissa,
            scale,
            discard: Discard::of_wide(remainder, denominator),
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
    read_parts(text).ok()
}

/// Why text a caller meant as a `decimal` is not one.
///
/// The two are told apart because the person who reads them is different: a
/// value past the range is read by whoever owns the schema it came out of, and
/// text that is not a literal is read by whoever owns the code that produced
/// it. [`Decimal::read`] is the member that answers this, and
/// [`Decimal::parse`] is the same walk for a caller with one answer for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotDecimal {
    /// Not an exact decimal literal in full — a spelling, not a magnitude.
    Unreadable,
    /// An exact decimal literal whose value needs more than 96 mantissa bits
    /// or a scale past 28. `rule:types/decimal` is where the bound is, and
    /// `Core\BigDecimal` is what lies past it.
    PastRange,
}

/// [`literal_parts`], with the two failures [`NotDecimal`] separates kept
/// apart: a spelling that is not a literal at all, and digits that are one and
/// are too many.
///
/// The split is drawn where the bytes stop being readable rather than where a
/// parse happens to overflow: a mantissa past a `u128` and an exponent past an
/// `i32` are both magnitudes, so they answer [`NotDecimal::PastRange`] even
/// though this function's own arithmetic is what refused them.
fn read_parts(text: &str) -> Result<(bool, u128, i32), NotDecimal> {
    let (negative, rest) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (numeric, power) = match rest.split_once(['e', 'E']) {
        Some((numeric, written)) => (numeric, exponent(written)?),
        None => (rest, 0),
    };
    let (whole, fraction) = numeric.split_once('.').unwrap_or((numeric, ""));
    if whole.is_empty() && fraction.is_empty() {
        return Err(NotDecimal::Unreadable);
    }
    if !whole
        .bytes()
        .chain(fraction.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return Err(NotDecimal::Unreadable);
    }
    let scale = i32::try_from(fraction.len())
        .ok()
        .and_then(|places| places.checked_sub(power))
        .ok_or(NotDecimal::PastRange)?;
    let mut digits = String::with_capacity(whole.len() + fraction.len());
    digits.push_str(whole);
    digits.push_str(fraction);
    let mantissa = digits.parse::<u128>().map_err(|_| NotDecimal::PastRange)?;
    Ok((negative, mantissa, scale))
}

/// The `e` half of a literal as its power of ten: a signed run of digits, and
/// nothing else. One past an `i32` is a magnitude rather than a spelling, so
/// it is [`NotDecimal::PastRange`] — no `decimal` has an exponent near that
/// bound, and neither has any `f64` this crate renders through [`f64_parts`].
fn exponent(written: &str) -> Result<i32, NotDecimal> {
    let digits = written.strip_prefix(['-', '+']).unwrap_or(written);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(NotDecimal::Unreadable);
    }
    written.parse().map_err(|_| NotDecimal::PastRange)
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
    fn a_decimal_division_past_128_bits_retries_at_192_and_is_exact() {
        // A 96-bit mantissa folded against a scale difference of 28 overflows
        // `u128` where the quotient still fits a `decimal`, on both sides: the
        // dividend's scale is the narrower one here, so the fold goes into the
        // numerator.
        let widest = Decimal::new(false, MAX_MANTISSA, 0).expect("at the bound");
        let one = Decimal::new(false, 10u128.pow(28), MAX_SCALE).expect("one, at full scale");
        assert_eq!(widest.checked_div(one), Some(widest));
        assert_eq!(widest.checked_div_exact(one), Some(widest));

        // Exactly half at the last digit the mantissa admits, rounded to the
        // even neighbour — 2^95, where truncating would answer 2^95 - 1.
        let two = Decimal::new(false, 2 * 10u128.pow(28), MAX_SCALE).expect("two, at full scale");
        assert_eq!(widest.checked_div(two), Decimal::new(false, 1u128 << 95, 0));
        assert_eq!(widest.checked_div_exact(two), None);

        // The other fold: the divisor's scale is the narrower one, so the
        // 10^28 goes into the denominator instead.
        let padded = dec("0.4500000000000000000000000000");
        assert_eq!(padded.scale(), MAX_SCALE);
        assert_eq!(
            padded.checked_div(dec("45000000000")),
            Decimal::new(false, 1, 11)
        );
        assert_eq!(
            padded.checked_div_at_scale(dec("45000000000"), 11),
            Some((
                Decimal::new(false, 1, 11).expect("in range"),
                Discard::Nothing
            ))
        );

        // A wider intermediate is not a wider `decimal`: a quotient past 96
        // bits is refused at 192 as it was at 128.
        let tiny = Decimal::new(false, 1, MAX_SCALE).expect("in range");
        assert_eq!(widest.checked_div(tiny), None);
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
