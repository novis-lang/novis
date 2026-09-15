//! `Core\Decimal` — `rule:types/arithmetic`'s named-rounding members: the two spellings a program reaches for when
//! rounding is business logic rather than an artifact of the operator.
//!
//! The `/` operator is already total over `decimal` and already rounds — half
//! to even, at the widest scale the result admits, fixed in the language and
//! not configurable, because an ambient precision read by unrelated later code
//! is the shape `rule:statements/static-is-a-member-modifier` and
//! `rule:security/no-cross-request-state` both close. What
//! this class adds is the questions that policy cannot answer:
//!
//! * **"this division must not lose anything"** — `divExact`, which throws
//!   rather than rounding, so a quotient that repeats is a caught error at the
//!   site that assumed it would not;
//! * **"round it here, this way"** — `divRound`, where the scale and the mode
//!   are both written out loud at the call, and neither is read from anywhere
//!   else;
//! * **"split this sum and lose none of it"** — `allocate`, where no rounding
//!   is named at all because there is none to name: the parts add back to the
//!   amount exactly, which is the one answer dividing each share separately
//!   cannot give.
//!
//! `pow` is here for a different reason. `**` has no row for a `decimal` base
//! at all (`rule:types/arithmetic`), because a power that does not come out
//! exact would have to round without having been asked to; the member answers
//! the exact power or throws, and a negative one is written as the division it
//! is.
//!
//! # One division, read three ways
//!
//! Neither member re-implements division. `nvs_runtime::decimal`'s
//! `long_divide` is the single long division; `Decimal::checked_div` is it plus
//! half-even, `Decimal::checked_div_exact` is it plus a refusal when anything
//! is left over, and `Decimal::checked_div_at_scale` is it stopped at a named
//! scale, handing back the `Discard` this module turns into a mode's decision.
//! That factoring is the point: three divisions that agree today would be the
//! bug, and `divRound` in particular must **not** be `checked_div` rounded a
//! second time — a quotient rounded at scale 28 and then again at scale 2
//! carries a tie into the second decision that the exact value never had.
//!
//! **`divRound` refuses at a scale the answer cannot hold rather than narrowing
//! to one it can.** That is the refusal `rule:types/arithmetic`'s `decimal ⊕ decimal` row
//! already states for a product whose scale would exceed 28, and the same
//! divergence from `System.Decimal`: silently narrowing would make a second
//! operation inexact without saying so.
//!
//! # Which class a refusal throws
//!
//! The divisions throw `ArithmeticError` for everything, including a zero
//! divisor, because `rule:types/arithmetic` fixes that class for the operator
//! they stand beside. `allocate` splits its refusals: a ratio list that is
//! empty, all zero or negative is a `LogicError`, since nothing about it is an
//! arithmetic that overflowed — it is a call that cannot mean anything, which
//! is the bad-argument shape `LogicError` is for. What `allocate` throws
//! `ArithmeticError` for is the same bound the operators have, a share wider
//! than a `decimal` holds.
//!
//! # Known gaps
//!
//! * **`floor`, `ceil`, `truncate` and `round` are not here yet.** ADR 0054
//!   § 4 names them as where a `decimal → int` conversion says its rounding
//!   out loud, and `crate::math`'s own gap note explains why they land on this
//!   class rather than widening `Core\Math`'s `float` ones. Each takes a
//!   target scale defaulting to 0, and `round` takes a `Core\RoundMode` with
//!   no default, naming the mode being the point.
//!   — owner: M8

use nvs_runtime::decimal::Discard;
use nvs_runtime::{Decimal, Fault, NvsArray, ThrownClass, Value};

use crate::math::{RoundMode, round_mode};
use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc};

/// `rule:types/decimal`'s named members in that rule's own order — the two
/// divisions that say their rounding out loud, then the split that has no
/// rounding left to name because its parts add back to the amount exactly —
/// and then the power `rule:types/arithmetic` sends a `decimal` base to
/// instead of `**`.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Decimal",
    methods: &[
        CoreMethod {
            name: "divExact",
            names: &["value", "divisor"],
            params: &[CoreTy::Decimal, CoreTy::Decimal],
            defaults: &[],
            return_ty: CoreTy::Decimal,
            symbol: "nvs_core_decimal_div_exact",
            doc: Some(&DIV_EXACT_DOC),
        },
        CoreMethod {
            name: "divRound",
            names: &["value", "divisor", "scale", "mode"],
            params: &[
                CoreTy::Decimal,
                CoreTy::Decimal,
                CoreTy::Uint,
                CoreTy::Enum(r"Core\RoundMode"),
            ],
            defaults: &[],
            return_ty: CoreTy::Decimal,
            symbol: "nvs_core_decimal_div_round",
            doc: Some(&DIV_ROUND_DOC),
        },
        CoreMethod {
            name: "allocate",
            names: &["amount", "ratios"],
            params: &[CoreTy::Decimal, CoreTy::Array(&CoreTy::Decimal)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Decimal),
            symbol: "nvs_core_decimal_allocate",
            doc: Some(&ALLOCATE_DOC),
        },
        CoreMethod {
            name: "pow",
            names: &["base", "exponent"],
            params: &[CoreTy::Decimal, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Decimal,
            symbol: "nvs_core_decimal_pow",
            doc: Some(&POW_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Decimal::divExact`'s reference card — `rule:core-api/reference-card`.
const DIV_EXACT_DOC: MethodDoc = MethodDoc {
    short: "`$value / $divisor` where the quotient is exact, and a throw where it is not — the \
            division for a place that has assumed the split comes out even, so the assumption \
            fails loudly instead of silently rounding.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The dividend.",
            shape: &[],
        },
        ParamDoc {
            name: "divisor",
            desc: "What to divide it by.",
            shape: &[],
        },
    ],
    ret: "The quotient, at the scale it needs and no wider — `1.00 / 4` is `0.25`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When `$divisor` is zero, when the quotient repeats, and when it is wider than a \
               `decimal` holds — the last two say so in one sentence, because either way the \
               exact answer is not available.",
    }],
};

/// `Core\Decimal::divRound`'s reference card — `rule:core-api/reference-card`.
const DIV_ROUND_DOC: MethodDoc = MethodDoc {
    short: "`$value / $divisor` rounded to `$scale` places under `$mode`, both named at the call \
            — the spelling for rounding that is business logic, since the `/` operator's own \
            half-even rule is fixed and takes no argument.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The dividend.",
            shape: &[],
        },
        ParamDoc {
            name: "divisor",
            desc: "What to divide it by.",
            shape: &[],
        },
        ParamDoc {
            name: "scale",
            desc: "How many digits after the point the answer keeps, at most 28. The answer \
                   carries exactly this scale, so a rounded price renders with its trailing \
                   zeros.",
            shape: &[],
        },
        ParamDoc {
            name: "mode",
            desc: "How a value between two neighbours settles — the same `Core\\RoundMode` \
                   `Core\\Math::round` takes, applied to the exact quotient rather than to an \
                   already-rounded one.",
            shape: &[],
        },
    ],
    ret: "The quotient at exactly `$scale` places.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When `$divisor` is zero, when `$scale` is past 28, and when the quotient's \
               mantissa would not fit at that scale.",
    }],
};

/// `Core\Decimal::allocate`'s reference card — `rule:core-api/reference-card`.
const ALLOCATE_DOC: MethodDoc = MethodDoc {
    short: "Splits `$amount` into one part per ratio, at the amount's own scale and adding back to \
            it exactly — the penny split, which dividing and rounding each share on its own \
            loses or invents a smallest unit of.",
    params: &[
        ParamDoc {
            name: "amount",
            desc: "The sum to split. Its scale is the parts' scale, so a price at two places is \
                   split into parts at two places.",
            shape: &[],
        },
        ParamDoc {
            name: "ratios",
            desc: "One weight per part, each zero or more, at least one of them above zero. The \
                   keys are kept, so a split written under the names of its parties answers \
                   under those names.",
            shape: &[],
        },
    ],
    ret: "One part per ratio, in the ratios' own order and under their own keys: each its share \
          truncated towards zero, and then one more smallest unit for each of the earliest parts \
          until what is left over is gone.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "When there are no ratios, when a ratio is negative, and when every ratio is \
                   zero — three ways of asking for a split that has no parts to make.",
        },
        ErrorDoc {
            error: "ArithmeticError",
            desc: "When a share is wider than a `decimal` holds: the amount and a ratio together \
                   want more than 28 fractional digits or more than a 96-bit mantissa.",
        },
    ],
};

/// `Core\Decimal::pow`'s reference card — `rule:core-api/reference-card`.
const POW_DOC: MethodDoc = MethodDoc {
    short: "`$base` multiplied by itself `$exponent` times, exactly — the power a `decimal` base \
            takes, since `**` has no row for one and would have to round to get an answer.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The value to raise.",
            shape: &[],
        },
        ParamDoc {
            name: "exponent",
            desc: "How many times to multiply it by itself. Zero answers `1`, and there is no \
                   negative exponent: `Core\\Decimal::divRound(1, Core\\Decimal::pow($b, $n), \
                   $scale, $mode)` is the reciprocal, with the rounding it needs written out.",
            shape: &[],
        },
    ],
    ret: "The exact power, at the scale repeated multiplication gives it — `2.50` squared is \
          `6.2500`, since a product's scale is its operands' scales added.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When the power is wider than a `decimal` holds: more than 28 fractional digits, \
               which a scaled base reaches quickly, or more than a 96-bit mantissa.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_decimal_div_exact" => (nvs_core_decimal_div_exact as *const ()).cast(),
        "nvs_core_decimal_div_round" => (nvs_core_decimal_div_round as *const ()).cast(),
        "nvs_core_decimal_allocate" => (nvs_core_decimal_allocate as *const ()).cast(),
        "nvs_core_decimal_pow" => (nvs_core_decimal_pow as *const ()).cast(),
        _ => return None,
    })
}

/// The `decimal` argument at `index`.
fn decimal_at(args: &[Value], index: usize, member: &str) -> Result<Decimal, Fault> {
    args[index].as_decimal().ok_or_else(|| {
        // Unreachable from source: both parameters are declared
        // `CoreTy::Decimal`, so anything else is
        // `E0401: expected `decimal`, found …` at the argument.
        Fault::fatal(format!(
            "Core\\Decimal::{member} expected a `decimal` at argument {index}, got tag {}",
            args[index].tag_byte()
        ))
    })
}

/// Whether a quotient truncated to `mantissa`, having thrown `discard` away,
/// rounds away from zero under `mode`.
///
/// The magnitude is what moves: the sign is carried beside the mantissa, so
/// "away from zero" is one increment either way and each mode is one rule
/// rather than two. `Core\Math::round`'s `apply` is the same six rules over an
/// `f64`, and the two cannot share code because that one has no remainder to
/// look at — it asks IEEE the same question.
fn rounds_away(mode: RoundMode, discard: Discard, mantissa: u128) -> bool {
    match discard {
        Discard::Nothing => false,
        Discard::BelowHalf => mode == RoundMode::Up,
        Discard::AboveHalf => mode != RoundMode::Down,
        Discard::Half => match mode {
            RoundMode::Up | RoundMode::HalfUp => true,
            RoundMode::Down | RoundMode::HalfDown => false,
            RoundMode::HalfEven => mantissa % 2 == 1,
            RoundMode::HalfOdd => mantissa.is_multiple_of(2),
        },
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Decimal::divExact(decimal $value, decimal $divisor): decimal` —
    /// `rule:types/arithmetic`'s "throws unless the quotient is exact".
    ///
    /// The zero divisor is separated from the rest because it is a different
    /// mistake: dividing by nothing is a bug in the program, while an
    /// inexact quotient is a bug in the assumption the call site made. Both
    /// are `ArithmeticError`, since a `catch` that wants to tell them apart
    /// wants the sentence rather than a seventh class.
    fn nvs_core_decimal_div_exact(_ctx, args: [2]) {
        let value = decimal_at(args, 0, "divExact")?;
        let divisor = decimal_at(args, 1, "divExact")?;
        if divisor.is_zero() {
            return Err(Fault::thrown_as(
                ThrownClass::Arithmetic,
                "Core\\Decimal::divExact cannot divide by zero".to_owned(),
            ));
        }
        let quotient = value.checked_div_exact(divisor).ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Arithmetic,
                format!(
                    "Core\\Decimal::divExact cannot answer {value} / {divisor} exactly: the \
                     quotient repeats or is wider than a `decimal` holds"
                ),
            )
        })?;
        Ok(Value::decimal(quotient))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Decimal::divRound(decimal $value, decimal $divisor, uint $scale,
    /// Core\RoundMode $mode): decimal` — `rule:types/arithmetic`'s "names both".
    ///
    /// Both arguments are required rather than optional, which is the one
    /// place this member parts from `rule:core-api/shape-rules` R3's trailing options shape: a
    /// default scale or a default mode would be exactly the ambient precision
    /// § 3 refuses, moved from a global into a signature. A caller who wants
    /// the language's own rule already has `/`.
    fn nvs_core_decimal_div_round(_ctx, args: [4]) {
        let value = decimal_at(args, 0, "divRound")?;
        let divisor = decimal_at(args, 1, "divRound")?;
        let scale = args[2].as_uint().unwrap_or(u64::from(u8::MAX));
        let mode = round_mode(args, 3, "Core\\Decimal::divRound")?;
        if divisor.is_zero() {
            return Err(Fault::thrown_as(
                ThrownClass::Arithmetic,
                "Core\\Decimal::divRound cannot divide by zero".to_owned(),
            ));
        }
        // One sentence for the scale bound and for a mantissa that will not
        // fit at it, because they are the same answer to the caller: no
        // `decimal` holds that quotient at that scale.
        let refused = || {
            Fault::thrown_as(
                ThrownClass::Arithmetic,
                format!(
                    "Core\\Decimal::divRound cannot answer {value} / {divisor} at scale \
                     {scale}: a `decimal` holds 28 fractional digits and a 96-bit mantissa"
                ),
            )
        };
        let places = u8::try_from(scale).map_err(|_| refused())?;
        let (truncated, discard) = value
            .checked_div_at_scale(divisor, places)
            .ok_or_else(refused)?;
        if !rounds_away(mode, discard, truncated.mantissa()) {
            return Ok(Value::decimal(truncated));
        }
        // The sign is recomputed rather than read back off `truncated`,
        // whose own is dropped where the mantissa is zero: `-0` is not
        // observable, but the increment below can make the magnitude
        // non-zero and the answer's sign is then the division's.
        let negative = value.is_negative() != divisor.is_negative();
        let away = truncated
            .mantissa()
            .checked_add(1)
            .and_then(|mantissa| Decimal::new(negative, mantissa, truncated.scale()))
            .ok_or_else(refused)?;
        Ok(Value::decimal(away))
    }
}

/// The `array<decimal>` argument at `index`, borrowed for the length of the
/// call — [`crate::arr::borrowed`] is why the handle is never dropped.
fn ratios_at(args: &[Value], index: usize) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    // Unreachable from source: the parameter is declared
    // `array<decimal>`, so anything else is
    // `E0401: expected `array<decimal>`, found …` at the argument.
    let array = args[index].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Decimal::allocate expected an `array` of ratios at argument {index}, got tag {}",
            args[index].tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// The ratio at `slot`, which every walk below reads the same way.
fn ratio_at(ratios: &NvsArray, slot: usize) -> Result<Decimal, Fault> {
    let value = ratios
        .value_at(slot)
        .expect("next_slot only names live entries");
    // Unreachable from source: the parameter is declared
    // `array<decimal>`, so an element of any other type is
    // `E0401: expected `decimal`, found …` where it is written.
    value.as_decimal().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Decimal::allocate expected a `decimal` ratio, got tag {}",
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Decimal::allocate(decimal $amount, array<decimal> $ratios):
    /// array<decimal>` — `rule:types/decimal`'s split whose parts add back to
    /// the amount exactly.
    ///
    /// The amount's own scale is the parts' scale, so the smallest unit the
    /// split can move is the one the amount is written in: a price at two
    /// places is split into cents, and a rate at four into hundredths of a
    /// cent. Each part starts as its exact share truncated towards zero,
    /// which leaves under one unit per part unallocated; those units are then
    /// handed out one each to the earliest parts, and there are always fewer
    /// of them than there are parts.
    ///
    /// **Position decides who gets one, not weight.** A ratio of zero takes
    /// no share of its own and is still a part, so it receives one of the
    /// leftover units when it comes first. That keeps the answer a function
    /// of the order the caller wrote rather than of a second comparison
    /// between the ratios, and a caller who means "nothing here at all"
    /// leaves the part out.
    ///
    /// The keys are the ratios', so a split written under the names of its
    /// parties answers under those names and a list answers a list.
    ///
    /// The magnitude is split and the sign put back afterwards, so the parts
    /// of `-0.05` are the negatives of `0.05`'s rather than a truncation
    /// leaning the other way.
    ///
    /// **What it spends:** one `Vec` of parts and the answer array, one entry
    /// per ratio each, both released with the call
    /// (`rule:programs/memory-priority`).
    fn nvs_core_decimal_allocate(_ctx, args: [2]) {
        let amount = decimal_at(args, 0, "allocate")?;
        let ratios = ratios_at(args, 1)?;
        let places = amount.scale();
        // One sentence for both overflow kinds, as `divRound` gives: either
        // way there is no `decimal` to answer with.
        let refused = || {
            Fault::thrown_as(
                ThrownClass::Arithmetic,
                format!(
                    "Core\\Decimal::allocate cannot split {amount} at scale {places} by these \
                     ratios: a `decimal` holds 28 fractional digits and a 96-bit mantissa"
                ),
            )
        };

        // The ratios are read three times rather than copied once: they are
        // an argument, so the walk is a borrow and the only array this member
        // allocates is the one it answers with.
        let mut total = Decimal::zero();
        let mut parts = 0usize;
        let mut from = 0usize;
        while let Some(slot) = ratios.next_slot(from) {
            from = slot + 1;
            let ratio = ratio_at(&ratios, slot)?;
            if ratio.is_negative() {
                return Err(Fault::thrown_as(
                    ThrownClass::Logic,
                    format!("Core\\Decimal::allocate has no share for the negative ratio {ratio}"),
                ));
            }
            total = total.checked_add(ratio).ok_or_else(refused)?;
            parts += 1;
        }
        // Two bad arguments rather than one, because they are two different
        // mistakes: no parts to make at all, and parts that every ratio asks
        // to be empty. Both are `LogicError` — the split is not an
        // arithmetic that overflowed but a call that cannot mean anything.
        if parts == 0 {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("Core\\Decimal::allocate needs at least one ratio to split {amount} by"),
            ));
        }
        if total.is_zero() {
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                format!("Core\\Decimal::allocate needs a ratio above zero to split {amount} by"),
            ));
        }

        // Both sides of the product are reduced first: the scales add, so a
        // ratio written `1.0` would otherwise spend one of the 28 on a digit
        // that carries nothing.
        let magnitude = amount.abs().reduced();
        let mut shares: Vec<Decimal> = Vec::with_capacity(parts);
        let mut allocated = Decimal::zero();
        let mut from = 0usize;
        while let Some(slot) = ratios.next_slot(from) {
            from = slot + 1;
            let ratio = ratio_at(&ratios, slot)?;
            // Multiplied before it is divided, so the share is the exact one
            // this ratio has of the whole rather than a rounded fraction
            // scaled up.
            let exact = magnitude.checked_mul(ratio.reduced()).ok_or_else(refused)?;
            let (share, _) = exact
                .checked_div_at_scale(total, places)
                .ok_or_else(refused)?;
            allocated = allocated.checked_add(share).ok_or_else(refused)?;
            shares.push(share);
        }

        // Every share carries the amount's scale and so does their sum, which
        // makes what is left over a whole number of smallest units: its
        // mantissa is that count, and the truncation above bounds it below
        // the number of parts.
        let remainder = magnitude.checked_sub(allocated).ok_or_else(refused)?;
        let unit = Decimal::new(false, 1, places).ok_or_else(refused)?;
        let mut owed = remainder.mantissa();
        for share in &mut shares {
            if owed == 0 {
                break;
            }
            *share = share.checked_add(unit).ok_or_else(refused)?;
            owed -= 1;
        }

        let negative = amount.is_negative();
        let mut shares = shares.into_iter();
        let mut out = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = ratios.next_slot(from) {
            from = slot + 1;
            let key = ratios
                .slot_key(slot)
                .expect("next_slot only names live entries");
            let share = shares
                .next()
                .expect("one share per ratio, and the same walk in the same order");
            let part = if negative { share.negated() } else { share };
            crate::arr::store_at(&mut out, key, Value::decimal(part));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Decimal::pow(decimal $base, uint $exponent): decimal` —
    /// `rule:types/arithmetic`'s row for a `decimal` base, which refuses `**`
    /// and names this instead.
    ///
    /// Exact or nothing, and the answer is what repeated multiplication
    /// gives **including its scale**, since a product's scale is its
    /// operands' added and scale is observable: `pow($b, 3)` is `$b * $b *
    /// $b` to the digit. A base with a scale of its own runs through the 28
    /// quickly, and reaching the end of them throws rather than narrowing to
    /// a scale that fits — the same divergence from `System.Decimal` the
    /// operator's own row makes.
    ///
    /// The exponent is a `uint` because a negative power is a division, and
    /// a division over `decimal` either comes out exact or names its
    /// rounding. The reciprocal is written `divRound(1, pow($b, $n), $scale,
    /// $mode)`, which says out loud what `$b ** -3` would have settled
    /// quietly.
    ///
    /// **Squared rather than walked**, so an exponent near `uint`'s ceiling
    /// is sixty-odd multiplications rather than a hang: a base of `1` reaches
    /// no bound at all, and every other base reaches one within those sixty.
    /// The answer is the walk's — each intermediate is the base to a power at
    /// or below the one asked for, so nothing overflows here that a walk
    /// would have carried through.
    fn nvs_core_decimal_pow(_ctx, args: [2]) {
        let base = decimal_at(args, 0, "pow")?;
        // Unreachable from source: the parameter is declared `uint`, so
        // anything else is `E0401: expected `uint`, found …` at the argument.
        let exponent = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Decimal::pow expected a `uint` exponent, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let refused = || {
            Fault::thrown_as(
                ThrownClass::Arithmetic,
                format!(
                    "Core\\Decimal::pow cannot answer {base} to the power {exponent}: a \
                     `decimal` holds 28 fractional digits and a 96-bit mantissa"
                ),
            )
        };

        let mut power = Decimal::new(false, 1, 0).expect("`1` is a `decimal`");
        let mut squared = base;
        let mut left = exponent;
        while left > 0 {
            if left % 2 == 1 {
                power = power.checked_mul(squared).ok_or_else(refused)?;
            }
            left /= 2;
            // Not squared past what was asked for: the last doubling would
            // be a power beyond the exponent, and refusing on *its* overflow
            // would refuse an answer that fits.
            if left > 0 {
                squared = squared.checked_mul(squared).ok_or_else(refused)?;
            }
        }
        Ok(Value::decimal(power))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, call};

    /// `Core\RoundMode`'s six cases, as the `int`s the enum's rows carry.
    const HALF_UP: i64 = 0;
    const HALF_DOWN: i64 = 1;
    const HALF_EVEN: i64 = 2;
    const HALF_ODD: i64 = 3;
    const UP: i64 = 4;
    const DOWN: i64 = 5;

    fn decimal(text: &str) -> Value {
        Value::decimal(Decimal::parse(text).expect("a decimal literal"))
    }

    /// A member end to end through the `rule:errors/propagation` boundary compiled code will
    /// reach it at, as `crate::math`'s own tests do.
    fn exact(ctx: &mut Ctx, value: &str, divisor: &str) -> Result<String, String> {
        answer(
            ctx,
            nvs_core_decimal_div_exact,
            &[decimal(value), decimal(divisor)],
        )
    }

    fn rounded(
        ctx: &mut Ctx,
        value: &str,
        divisor: &str,
        scale: u64,
        mode: i64,
    ) -> Result<String, String> {
        answer(
            ctx,
            nvs_core_decimal_div_round,
            &[
                decimal(value),
                decimal(divisor),
                Value::uint(scale),
                Value::int(mode),
            ],
        )
    }

    /// The rendered answer, or the sentence the throw carried — which is what
    /// a `catch` in a program reads, so it is what these assertions compare.
    fn answer(
        ctx: &mut Ctx,
        function: nvs_runtime::NvsFn,
        args: &[Value],
    ) -> Result<String, String> {
        match call(function, ctx, args) {
            Ok(value) => Ok(value
                .as_decimal()
                .expect("both members answer a `decimal`")
                .to_string()),
            Err(_) => Err(ctx
                .take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()),
        }
    }

    /// `rule:types/arithmetic`'s two named-rounding members over the same divisions:
    /// where `divExact` refuses, `divRound` answers under the mode it was
    /// given, and where `divExact` answers the two agree.
    #[test]
    fn div_exact_throws_where_div_round_rounds() {
        let mut ctx = Ctx::buffered();

        // A quotient that repeats: the whole reason the two members exist.
        let repeating = exact(&mut ctx, "1.00", "3").expect_err("one third repeats");
        assert!(
            repeating.contains("cannot answer 1.00 / 3 exactly"),
            "the refusal names the division that could not be exact: {repeating}"
        );
        assert_eq!(
            rounded(&mut ctx, "1.00", "3", 4, HALF_UP).as_deref(),
            Ok("0.3333"),
            "the same division at a scale the caller named"
        );

        // The tie every `Half*` mode is named for, at a scale that produces
        // one: 1/8 is 0.125 exactly, so rounding to two places is exactly
        // half and the six modes part.
        assert_eq!(exact(&mut ctx, "1", "8").as_deref(), Ok("0.125"));
        for (mode, want) in [
            (HALF_UP, "0.13"),
            (HALF_DOWN, "0.12"),
            (HALF_EVEN, "0.12"),
            (HALF_ODD, "0.13"),
            (UP, "0.13"),
            (DOWN, "0.12"),
        ] {
            assert_eq!(
                rounded(&mut ctx, "1", "8", 2, mode).as_deref(),
                Ok(want),
                "mode {mode} over an exact half"
            );
        }

        // Where the quotient is exact at the named scale, every mode agrees
        // with `divExact` — nothing is discarded, so there is no decision to
        // make and `Up` does not invent one.
        assert_eq!(exact(&mut ctx, "1.00", "4").as_deref(), Ok("0.25"));
        for mode in [HALF_UP, HALF_EVEN, UP, DOWN] {
            assert_eq!(
                rounded(&mut ctx, "1.00", "4", 2, mode).as_deref(),
                Ok("0.25"),
                "mode {mode} over a quotient that needs no rounding"
            );
        }
        // ...and the named scale is the answer's, so it pads rather than
        // reduces: `rule:types/conversion` makes scale observable.
        assert_eq!(
            rounded(&mut ctx, "1.00", "4", 5, HALF_EVEN).as_deref(),
            Ok("0.25000")
        );

        // A zero divisor is the one refusal both make, and each says its own
        // name.
        assert_eq!(
            exact(&mut ctx, "1", "0").expect_err("nothing divides by zero"),
            "Core\\Decimal::divExact cannot divide by zero"
        );
        assert_eq!(
            rounded(&mut ctx, "1", "0", 2, HALF_UP).expect_err("nor here"),
            "Core\\Decimal::divRound cannot divide by zero"
        );

        // A scale past the type's own bound is refused rather than clamped.
        let wide = rounded(&mut ctx, "1", "3", 40, HALF_UP).expect_err("28 is the bound");
        assert!(
            wide.contains("at scale 40"),
            "the refusal names the scale it was asked for: {wide}"
        );
    }

    /// `Core\Decimal::pow($base, $exponent)`'s answer, or the sentence its
    /// throw carried.
    fn power(ctx: &mut Ctx, base: &str, exponent: u64) -> Result<String, String> {
        answer(
            ctx,
            nvs_core_decimal_pow,
            &[decimal(base), Value::uint(exponent)],
        )
    }

    /// `rule:types/arithmetic`'s power for a `decimal` base: exact, at the
    /// scale repeated multiplication gives it, or a throw at either bound.
    #[test]
    fn decimal_pow_is_exact_or_throws_at_the_mantissa_or_scale_bound() {
        let mut ctx = Ctx::buffered();

        // A product's scale is its operands' added, so a squared `2.50` is
        // `6.2500` — the power agrees with `*` on the digits it renders as
        // well as on the value.
        assert_eq!(power(&mut ctx, "2.50", 2).as_deref(), Ok("6.2500"));
        assert_eq!(power(&mut ctx, "2", 10).as_deref(), Ok("1024"));
        assert_eq!(power(&mut ctx, "1.05", 2).as_deref(), Ok("1.1025"));

        // The exponent's own edges, and the one that says the exponent is not
        // walked: a base of `1` reaches no bound, so the largest `uint` there
        // is has to answer rather than run.
        assert_eq!(power(&mut ctx, "19.90", 0).as_deref(), Ok("1"));
        assert_eq!(power(&mut ctx, "19.90", 1).as_deref(), Ok("19.90"));
        assert_eq!(power(&mut ctx, "0", 5).as_deref(), Ok("0"));
        assert_eq!(power(&mut ctx, "1", u64::MAX).as_deref(), Ok("1"));

        // The scale bound: `1.05` carries two fractional digits, so its
        // fifteenth power wants thirty of them and no `decimal` has thirty.
        let scale = power(&mut ctx, "1.05", 15).expect_err("28 is the bound");
        assert!(
            scale.contains("1.05 to the power 15"),
            "the refusal names the power it could not answer: {scale}"
        );

        // The mantissa bound, from the side that has no fractional digit at
        // all: 10^28 fits in 96 bits and 10^29 does not.
        assert_eq!(
            power(&mut ctx, "10", 28).as_deref(),
            Ok("10000000000000000000000000000")
        );
        let mantissa = power(&mut ctx, "10", 29).expect_err("96 bits is the other bound");
        assert!(
            mantissa.contains("10 to the power 29"),
            "the refusal names the power it could not answer: {mantissa}"
        );
    }

    /// The `array<decimal>` of ratios a compiled call hands `allocate`.
    fn ratios(weights: &[&str]) -> Value {
        let mut array = NvsArray::new();
        for weight in weights {
            array.append(decimal(weight));
        }
        Value::array(array)
    }

    /// Releases a reference this test frame owns.
    fn released(value: Value) {
        #[expect(
            unsafe_code,
            reason = "a test frame owns exactly the reference it built or was answered with"
        )]
        unsafe {
            value.release();
        }
    }

    /// `Core\Decimal::allocate($amount, $weights)`'s parts, or the sentence
    /// its throw carried — the same boundary [`answer`] reads the divisions
    /// at, over an array rather than a scalar.
    fn split(ctx: &mut Ctx, amount: &str, weights: &[&str]) -> Result<Vec<Decimal>, String> {
        let list = ratios(weights);
        let parts = match call(nvs_core_decimal_allocate, ctx, &[decimal(amount), list]) {
            Ok(value) => {
                let answer =
                    crate::arr::borrowed(value.array_ptr().expect("`allocate` answers an array"));
                let mut parts = Vec::new();
                let mut from = 0usize;
                while let Some(slot) = answer.next_slot(from) {
                    from = slot + 1;
                    parts.push(
                        answer
                            .value_at(slot)
                            .expect("next_slot only names live entries")
                            .as_decimal()
                            .expect("every part is a `decimal`"),
                    );
                }
                released(value);
                Ok(parts)
            }
            Err(_) => Err(ctx
                .take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()),
        };
        released(list);
        parts
    }

    /// The parts as a program would see them printed, so an assertion reads
    /// as the answer rather than as a vector of it.
    fn rendered(parts: &[Decimal]) -> String {
        parts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// `rule:types/decimal`'s "splits a sum into parts that add back to it
    /// exactly", over the splits that have no even answer.
    #[test]
    fn decimal_allocate_parts_add_back_to_the_amount_exactly() {
        let mut ctx = Ctx::buffered();

        // The penny split: a third of five cents is not a number of cents, so
        // the cent left over goes to the earliest part and no part is a
        // fraction of one.
        let nickel = split(&mut ctx, "0.05", &["1", "1", "1"]).expect("three equal ratios");
        assert_eq!(rendered(&nickel), "0.02 0.02 0.01");

        // Weights that do come out even, and an amount whose scale is wider
        // than money's — the parts carry the amount's scale either way.
        assert_eq!(
            rendered(&split(&mut ctx, "100.00", &["7", "3"]).expect("a 70/30 split")),
            "70.00 30.00"
        );
        assert_eq!(
            rendered(&split(&mut ctx, "1.0000", &["1", "1", "1"]).expect("three equal ratios")),
            "0.3334 0.3333 0.3333"
        );

        // A ratio of zero takes no share of its own and is still a part, so
        // it can take a leftover unit: position decides that, not weight.
        assert_eq!(
            rendered(&split(&mut ctx, "0.05", &["0", "1", "1"]).expect("one ratio above zero")),
            "0.01 0.02 0.02"
        );

        // The magnitude is what is split, so a negative amount answers the
        // negatives of the positive one's parts rather than a truncation
        // leaning the other way.
        assert_eq!(
            rendered(&split(&mut ctx, "-0.05", &["1", "1", "1"]).expect("three equal ratios")),
            "-0.02 -0.02 -0.01"
        );

        // The property itself, over every split above and three more: the
        // parts add back to the amount exactly, at the amount's own scale,
        // and there is one of them per ratio.
        for (amount, weights) in [
            ("0.05", &["1", "1", "1"][..]),
            ("100.00", &["7", "3"]),
            ("1.0000", &["1", "1", "1"]),
            ("0.05", &["0", "1", "1"]),
            ("-0.05", &["1", "1", "1"]),
            ("19.99", &["0.3", "0.7"]),
            ("0.00", &["1", "2"]),
            ("7", &["1", "1", "1"]),
        ] {
            let parts = split(&mut ctx, amount, weights).expect("a ratio above zero");
            assert_eq!(parts.len(), weights.len(), "one part per ratio of {amount}");
            let mut back = Decimal::zero();
            for part in &parts {
                back = back
                    .checked_add(*part)
                    .expect("parts of a sum fit in the sum");
            }
            assert_eq!(
                back.to_string(),
                amount,
                "{amount} split {weights:?} adds back to itself"
            );
        }
    }

    /// The three ratio lists that name no split, and the one bound the split
    /// shares with the operators.
    #[test]
    fn decimal_allocate_refuses_an_empty_zero_or_negative_ratio_list() {
        let mut ctx = Ctx::buffered();

        assert_eq!(
            split(&mut ctx, "1.00", &[]).expect_err("no parts to make"),
            "Core\\Decimal::allocate needs at least one ratio to split 1.00 by"
        );
        assert_eq!(
            split(&mut ctx, "1.00", &["0", "0"]).expect_err("every part asks for nothing"),
            "Core\\Decimal::allocate needs a ratio above zero to split 1.00 by"
        );
        assert_eq!(
            split(&mut ctx, "1.00", &["2", "-1"]).expect_err("a part cannot owe"),
            "Core\\Decimal::allocate has no share for the negative ratio -1"
        );

        // `rule:types/decimal`'s 28 fractional digits, reached from the side
        // the split has of its own: an amount already at the bound and a
        // ratio with any scale at all want one more digit than the type has.
        let wide = split(&mut ctx, "0.0000000000000000000000000001", &["0.5", "0.5"])
            .expect_err("no `decimal` holds that share");
        assert!(
            wide.contains("at scale 28"),
            "the refusal names the scale it could not answer at: {wide}"
        );
    }

    /// Rounding moves the magnitude, and the sign is the division's — which
    /// is not the truncated quotient's, because a truncation to zero has no
    /// sign to read back off it.
    #[test]
    fn a_rounded_division_carries_the_signs_the_operands_gave_it() {
        let mut ctx = Ctx::buffered();
        // "Away from zero" is away in both directions, at the tie and above.
        assert_eq!(
            rounded(&mut ctx, "-1", "8", 2, HALF_UP).as_deref(),
            Ok("-0.13")
        );
        assert_eq!(
            rounded(&mut ctx, "-1", "8", 2, DOWN).as_deref(),
            Ok("-0.12")
        );
        assert_eq!(
            rounded(&mut ctx, "1", "-8", 2, UP).as_deref(),
            Ok("-0.13"),
            "one negative operand is one negative answer"
        );
        assert_eq!(
            rounded(&mut ctx, "-1", "-8", 2, UP).as_deref(),
            Ok("0.13"),
            "and two are none"
        );
        // The case the sign cannot be read back from: -1/1000 truncates to
        // 0.00, which has no sign at all, and `Up` then makes it -0.01.
        assert_eq!(
            rounded(&mut ctx, "-1", "1000", 2, UP).as_deref(),
            Ok("-0.01")
        );
        assert_eq!(
            rounded(&mut ctx, "-1", "1000", 2, DOWN).as_deref(),
            Ok("0.00"),
            "a zero has no sign to keep — `rule:types/decimal` has no `-0`"
        );
    }
}
