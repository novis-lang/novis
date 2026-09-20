//! `Core\BigInt` — an integer of any magnitude, immutable and method-based,
//! which is what [ADR 0054](/docs/decisions/0054.md) § 5 named as the
//! replacement for `gmp` and the integer half of `bcmath`.
//!
//! The `decimal` scalar beside it ([`crate::decimal`]) is the *human-magnitude*
//! answer — money, exact to 28 fractional digits — and this is the other one: a
//! factorial past `2^63`, `bcpowmod`, a power with a large exponent. Neither
//! subsumes the other, and `docs/spec/01-core-library.md` § 13's row names the
//! pair together.
//!
//! # Decision: the representation is a sign and little-endian magnitude bytes
//!
//! A value is an ordinary `Core`-owned instance under [`crate::instance`]'s
//! first decision — two slots, `sign` as an `int` in `{-1, 0, 1}` and
//! `magnitude` as `bytes` in [`num_bigint::BigUint::to_bytes_le`]'s own form.
//! Every member reads its operands back into a [`BigInt`], computes, and builds
//! a **new** instance, exactly as `decimal` arithmetic answers a new scalar:
//! there is no mutating member and no member that answers its receiver.
//!
//! The rejected alternative was a decimal-text `string` slot, which would have
//! made every operand a radix-10 parse rather than a copy. It stays written
//! down because it is the fallback if a slot ever cannot hold `bytes`.
//!
//! **What it spends:** one object plus one magnitude allocation per result
//! value, and one magnitude copy per operand read, all charged to the request
//! that produced them and released with it. [`nvs_runtime::affordable`] accounts
//! every magnitude against the request's ceiling before it is built, so a loop
//! that squares its way upward meets the request's memory cap rather than the
//! host's. A `BigInt` is on no served request's hot path by design; a program
//! that puts one there pays what it asked for.
//!
//! # Decision: the two members the engine reaches by name are rows like any other
//!
//! `toString` renders in radix 10 and `compareTo` orders two values, so
//! [`crate::instance::descriptors`] derives this class's `renderer` and
//! `comparer` from the registry the way it does for every other `Core` class —
//! `echo`, `<=>`, `<`, `>`, `<=`, `>=` and `Core\Arr::sort` therefore follow
//! `rule:classes/stringable` and `rule:classes/comparable` with no rule of this
//! class's own. The sort is in that list because [`crate::ordering`]'s natural
//! order sends two objects to `rule:classes/comparable`'s `compareTo`, so a
//! sort over these needs no `{comparator: ...}`.
//!
//! **There are no operators.** `$a + $b` over two objects is the compile error
//! it is for any class, because Novis has no operator overloading (ADR 0054's
//! own ground), and `nvs convert` maps `bcadd` and `bcpowmod` to [`CLASS`]'s
//! `add` and `powMod` rather than to a spelling that does not exist.
//!
//! **`parse` is not `rule:expressions/try-parse`'s member**, and nothing here
//! is owed a `tryParse` because of it: that rule's `parse` takes one `string`
//! and nothing else, and this one carries a radix bag beside the text, so it
//! answers a different question. A caller that wants the nullable half writes
//! the `catch`.
//!
//! # Decision: one call may not explode, and two members are where that is checked
//!
//! `add`, `mul` and their neighbours grow a result by at most the sum of their
//! operands' widths, so a program reaches a memory ceiling through
//! [`nvs_runtime::affordable`] a step at a time. `pow` and `shl` are the two
//! that turn a small receiver into an arbitrarily wide answer in **one** call,
//! and both refuse ahead of the work at [`MAX_BITS`] rather than allocating
//! toward the cap first. That bound is this module's, recorded here because
//! ADR 0054 left it to whoever built the class.
//!
//! The refusal classes are `decimal`'s, for `decimal`'s reasons:
//! `ArithmeticError` for a zero divisor, a zero modulus, a negative `sqrt`, a
//! width past the bound and every conversion out of range; `ParseError` for
//! text that is not an integer in the radix asked for; `LogicError` for a radix
//! outside 2 to 36, which is a bug in the program rather than in its input.

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use nvs_runtime::{Decimal, Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\BigInt`'s fully-qualified name, written once — [`CLASS`] declares it
/// and every [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
pub(crate) const NAME: &str = r"Core\BigInt";

/// The sign slot: `-1`, `0` or `1`, which is also what `sign` answers.
const SIGN_SLOT: usize = 0;

/// The magnitude slot: the absolute value's bytes, least significant first.
const MAGNITUDE_SLOT: usize = 1;

/// The narrowest and widest radix `parse` reads and `format` writes — digits
/// `0`-`9` then `a`-`z`, which is `strtol`'s range and PHP's `base_convert`'s.
const MIN_RADIX: u64 = 2;

/// The widest radix, [`MIN_RADIX`]'s other end.
const MAX_RADIX: u64 = 36;

/// The radix every member that takes one falls back to.
const DEFAULT_RADIX: u64 = 10;

/// The widest magnitude a single call may produce, in bits — 128 KiB of
/// magnitude, which is a 315,653-digit decimal number.
///
/// Only `pow` and `shl` are checked against it, for this module's own
/// § *Decision: one call may not explode*: they are the two members whose
/// answer is not bounded by the width of what was handed to them.
const MAX_BITS: u64 = 1 << 20;

/// Spec § 13's `Core\BigInt` — construction, arithmetic, and reading a value
/// back out, in that order.
///
/// The roster is this module's own rather than the spec's: § 13's row names the
/// class and no member, so `docs/agent/goals/` is where it was decided and each
/// row's reference card is what documents it.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "of",
            names: &["value"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_of",
            doc: Some(&OF_DOC),
        },
        CoreMethod {
            name: "ofUint",
            names: &["value"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_of_uint",
            doc: Some(&OF_UINT_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Options(RADIX_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_parse",
            doc: Some(&PARSE_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "add",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_add",
            doc: Some(&ADD_DOC),
        },
        CoreMethod {
            name: "sub",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_sub",
            doc: Some(&SUB_DOC),
        },
        CoreMethod {
            name: "mul",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_mul",
            doc: Some(&MUL_DOC),
        },
        CoreMethod {
            name: "div",
            names: &["divisor"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_div",
            doc: Some(&DIV_DOC),
        },
        CoreMethod {
            name: "mod",
            names: &["divisor"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_mod",
            doc: Some(&MOD_DOC),
        },
        CoreMethod {
            name: "pow",
            names: &["exponent"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_pow",
            doc: Some(&POW_DOC),
        },
        CoreMethod {
            name: "powMod",
            names: &["exponent", "modulus"],
            params: &[CoreTy::Instance(NAME), CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_pow_mod",
            doc: Some(&POW_MOD_DOC),
        },
        CoreMethod {
            name: "sqrt",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_sqrt",
            doc: Some(&SQRT_DOC),
        },
        CoreMethod {
            name: "gcd",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_gcd",
            doc: Some(&GCD_DOC),
        },
        CoreMethod {
            name: "lcm",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_lcm",
            doc: Some(&LCM_DOC),
        },
        CoreMethod {
            name: "neg",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_neg",
            doc: Some(&NEG_DOC),
        },
        CoreMethod {
            name: "abs",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_abs",
            doc: Some(&ABS_DOC),
        },
        CoreMethod {
            name: "shl",
            names: &["bits"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_shl",
            doc: Some(&SHL_DOC),
        },
        CoreMethod {
            name: "shr",
            names: &["bits"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_bigint_shr",
            doc: Some(&SHR_DOC),
        },
        CoreMethod {
            name: "sign",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_bigint_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_bigint_compare_to",
            doc: Some(&COMPARE_TO_DOC),
        },
        CoreMethod {
            name: "toInt",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_bigint_to_int",
            doc: Some(&TO_INT_DOC),
        },
        CoreMethod {
            name: "toUint",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_bigint_to_uint",
            doc: Some(&TO_UINT_DOC),
        },
        CoreMethod {
            name: "toDecimal",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Decimal,
            symbol: "nvs_core_bigint_to_decimal",
            doc: Some(&TO_DECIMAL_DOC),
        },
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_bigint_to_string",
            doc: Some(&TO_STRING_DOC),
        },
        CoreMethod {
            name: "format",
            names: &[],
            params: &[CoreTy::Options(RADIX_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_bigint_format",
            doc: Some(&FORMAT_DOC),
        },
    ],
    slots: &["sign", "magnitude"],
    constants: &[],
};

/// The `{radix?: uint}` both `parse` and `format` carry.
///
/// `rule:core-api/shape-rules` R2 makes every option optional, so the radix is
/// written with the default every other spelling in the language already means
/// by a bare number rather than as a demand a call site must answer.
const RADIX_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "radix",
    ty: CoreTy::Uint,
    default: Const::Uint(DEFAULT_RADIX),
}];

/// `Core\BigInt::of`'s reference card — `rule:core-api/reference-card`.
const OF_DOC: MethodDoc = MethodDoc {
    short: "Builds a `BigInt` holding exactly `$value` — the widening every program that grows \
            past `int` starts from.",
    params: &[ParamDoc {
        name: "value",
        desc: "The `int` to widen; every one of them, including `int`'s own bounds, is exact.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to `$value`.",
    errors: &[],
};

/// `Core\BigInt::ofUint`'s reference card — `rule:core-api/reference-card`.
const OF_UINT_DOC: MethodDoc = MethodDoc {
    short: "Builds a `BigInt` holding exactly `$value` — `of` over the unsigned scalar, whose \
            upper half no `int` can carry.",
    params: &[ParamDoc {
        name: "value",
        desc: "The `uint` to widen; the whole range is exact.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to `$value`.",
    errors: &[],
};

/// `Core\BigInt::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads `$text` as an integer of any magnitude, in the radix asked for — the text \
            `as int` reads, with no bound on how large it may be.",
    params: &[
        ParamDoc {
            name: "text",
            desc: "The whole text to read: an optional `-` or `+`, then digits, and nothing \
                   else. No leading-garbage rule, and no `_` separators — those are an integer \
                   literal's spelling rather than a `string`'s.",
            shape: &[],
        },
        ParamDoc {
            name: "radix",
            desc: "The base the digits are written in, 2 to 36 with `a`-`z` as the digits past \
                   `9`; 10 when the option is omitted.",
            shape: &[],
        },
    ],
    ret: "A `BigInt` equal to the number `$text` spells.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "`$text` is not an integer in that radix — it is empty, carries a separator, \
                   or holds a digit the radix does not have.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`radix` is outside 2 to 36.",
        },
    ],
};

/// `Core\BigInt::add`'s reference card — `rule:core-api/reference-card`.
const ADD_DOC: MethodDoc = MethodDoc {
    short: "The sum of this value and `$other`, as a new `BigInt` — there is no magnitude at \
            which it overflows.",
    params: &[ParamDoc {
        name: "other",
        desc: "The value to add.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to the sum.",
    errors: &[],
};

/// `Core\BigInt::sub`'s reference card — `rule:core-api/reference-card`.
const SUB_DOC: MethodDoc = MethodDoc {
    short: "The difference between this value and `$other`, as a new `BigInt`.",
    params: &[ParamDoc {
        name: "other",
        desc: "The value to subtract.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to the difference.",
    errors: &[],
};

/// `Core\BigInt::mul`'s reference card — `rule:core-api/reference-card`.
const MUL_DOC: MethodDoc = MethodDoc {
    short: "The product of this value and `$other`, as a new `BigInt` — the member a factorial \
            or a running power is written with.",
    params: &[ParamDoc {
        name: "other",
        desc: "The value to multiply by.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to the product.",
    errors: &[],
};

/// `Core\BigInt::div`'s reference card — `rule:core-api/reference-card`.
const DIV_DOC: MethodDoc = MethodDoc {
    short: "The quotient of this value and `$divisor`, truncated toward zero exactly as \
            `intdiv` truncates an `int` one.",
    params: &[ParamDoc {
        name: "divisor",
        desc: "The value to divide by.",
        shape: &[],
    }],
    ret: "A `BigInt` quotient; `-7 div 2` is `-3`, the answer `intdiv` gives.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "`$divisor` is zero.",
    }],
};

/// `Core\BigInt::mod`'s reference card — `rule:core-api/reference-card`.
const MOD_DOC: MethodDoc = MethodDoc {
    short: "The remainder of this value divided by `$divisor`, carrying the dividend's sign \
            exactly as `%` on `int` does.",
    params: &[ParamDoc {
        name: "divisor",
        desc: "The value to divide by.",
        shape: &[],
    }],
    ret: "A `BigInt` remainder; `-7 mod 2` is `-1`, so `div` and `mod` rebuild the dividend.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "`$divisor` is zero.",
    }],
};

/// `Core\BigInt::pow`'s reference card — `rule:core-api/reference-card`.
const POW_DOC: MethodDoc = MethodDoc {
    short: "This value raised to `$exponent`, which is the member `bcpow` with a large exponent \
            becomes.",
    params: &[ParamDoc {
        name: "exponent",
        desc: "The power to raise this value to; `0` answers `1` for every receiver.",
        shape: &[],
    }],
    ret: "A `BigInt` equal to the power.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "The answer would be wider than 1 048 576 bits, which this member refuses before \
               computing rather than after allocating.",
    }],
};

/// `Core\BigInt::powMod`'s reference card — `rule:core-api/reference-card`.
const POW_MOD_DOC: MethodDoc = MethodDoc {
    short: "This value raised to `$exponent`, modulo `$modulus`, computed without ever holding \
            the whole power — `bcpowmod`, and the member every modular exponentiation wants.",
    params: &[
        ParamDoc {
            name: "exponent",
            desc: "The power to raise this value to; it may not be negative.",
            shape: &[],
        },
        ParamDoc {
            name: "modulus",
            desc: "The modulus the answer is reduced by.",
            shape: &[],
        },
    ],
    ret: "A `BigInt` in `[0, |$modulus|)` for a non-negative receiver.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "`$modulus` is zero, or `$exponent` is negative.",
    }],
};

/// `Core\BigInt::sqrt`'s reference card — `rule:core-api/reference-card`.
const SQRT_DOC: MethodDoc = MethodDoc {
    short: "The integer square root of this value — the largest `BigInt` whose square does not \
            exceed it.",
    params: &[],
    ret: "A `BigInt` floor of the square root; `8` answers `2`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "This value is negative, which has no integer square root.",
    }],
};

/// `Core\BigInt::gcd`'s reference card — `rule:core-api/reference-card`.
const GCD_DOC: MethodDoc = MethodDoc {
    short: "The greatest common divisor of this value and `$other`, never negative — \
            `Core\\Math::gcd` at any magnitude.",
    params: &[ParamDoc {
        name: "other",
        desc: "The other value; `gcd(0, 0)` is `0`.",
        shape: &[],
    }],
    ret: "A non-negative `BigInt`.",
    errors: &[],
};

/// `Core\BigInt::lcm`'s reference card — `rule:core-api/reference-card`.
const LCM_DOC: MethodDoc = MethodDoc {
    short: "The least common multiple of this value and `$other`, never negative — \
            `Core\\Math::lcm` at any magnitude.",
    params: &[ParamDoc {
        name: "other",
        desc: "The other value; `lcm` with a zero operand is `0`.",
        shape: &[],
    }],
    ret: "A non-negative `BigInt`.",
    errors: &[],
};

/// `Core\BigInt::neg`'s reference card — `rule:core-api/reference-card`.
const NEG_DOC: MethodDoc = MethodDoc {
    short: "This value with its sign flipped — the spelling unary `-` would be if this class \
            had operators.",
    params: &[],
    ret: "A `BigInt` of the same magnitude and the opposite sign; zero answers zero.",
    errors: &[],
};

/// `Core\BigInt::abs`'s reference card — `rule:core-api/reference-card`.
const ABS_DOC: MethodDoc = MethodDoc {
    short: "This value's magnitude, with no bound to overflow at — unlike `abs` on `int`, whose \
            most negative value has no positive twin.",
    params: &[],
    ret: "A non-negative `BigInt`.",
    errors: &[],
};

/// `Core\BigInt::shl`'s reference card — `rule:core-api/reference-card`.
const SHL_DOC: MethodDoc = MethodDoc {
    short: "This value multiplied by two to the `$bits` — the shift, written as a member \
            because `<<` over an object does not parse.",
    params: &[ParamDoc {
        name: "bits",
        desc: "How many bits to shift by.",
        shape: &[],
    }],
    ret: "A `BigInt` `$bits` wider than this one.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "The answer would be wider than 1 048 576 bits.",
    }],
};

/// `Core\BigInt::shr`'s reference card — `rule:core-api/reference-card`.
const SHR_DOC: MethodDoc = MethodDoc {
    short: "This value divided by two to the `$bits`, rounding toward negative infinity as an \
            arithmetic shift does.",
    params: &[ParamDoc {
        name: "bits",
        desc: "How many bits to shift by; past this value's width the answer is `0` or `-1`.",
        shape: &[],
    }],
    ret: "A `BigInt` `$bits` narrower than this one.",
    errors: &[],
};

/// `Core\BigInt::sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "This value's sign, as the three-way answer `compareTo` gives against zero.",
    params: &[],
    ret: "`-1` below zero, `0` at it, `1` above.",
    errors: &[],
};

/// `Core\BigInt::compareTo`'s reference card — `rule:core-api/reference-card`.
const COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Orders this value against `$other` — `Comparable`'s member, so `<`, `<=>` and \
            `Core\\Arr::sort` answer through it.",
    params: &[ParamDoc {
        name: "other",
        desc: "The value to order against.",
        shape: &[],
    }],
    ret: "`-1` when this value is the smaller, `0` when they are equal, `1` when it is larger.",
    errors: &[],
};

/// `Core\BigInt::toInt`'s reference card — `rule:core-api/reference-card`.
const TO_INT_DOC: MethodDoc = MethodDoc {
    short: "This value as an `int`, exactly or not at all — the narrowing back down, which is \
            where a magnitude that outgrew `int` is discovered rather than truncated.",
    params: &[],
    ret: "The `int` equal to this value.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "This value is outside the `int` range.",
    }],
};

/// `Core\BigInt::toUint`'s reference card — `rule:core-api/reference-card`.
const TO_UINT_DOC: MethodDoc = MethodDoc {
    short: "This value as a `uint`, exactly or not at all — `toInt` over the unsigned scalar, \
            which reaches higher and refuses every negative value.",
    params: &[],
    ret: "The `uint` equal to this value.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "This value is negative or above the `uint` range.",
    }],
};

/// `Core\BigInt::toDecimal`'s reference card — `rule:core-api/reference-card`.
const TO_DECIMAL_DOC: MethodDoc = MethodDoc {
    short: "This value as a `decimal` at scale zero, which is exact wherever the magnitude fits \
            the scalar's 96-bit mantissa.",
    params: &[],
    ret: "A `decimal` equal to this value.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "This value needs more than 96 bits, which no `decimal` holds.",
    }],
};

/// `Core\BigInt::toString`'s reference card — `rule:core-api/reference-card`.
const TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "This value in radix 10, with a leading `-` when it is negative — `Stringable`'s \
            member, so `echo` and string interpolation render through it.",
    params: &[],
    ret: "The decimal digits, which `parse` reads back to the same value.",
    errors: &[],
};

/// `Core\BigInt::format`'s reference card — `rule:core-api/reference-card`.
const FORMAT_DOC: MethodDoc = MethodDoc {
    short: "This value in the radix asked for, using `a`-`z` for the digits past `9` — \
            `toString` in any base, and `parse`'s inverse at every one of them.",
    params: &[ParamDoc {
        name: "radix",
        desc: "The base to write in, 2 to 36; 10 when the option is omitted.",
        shape: &[],
    }],
    ret: "The digits, with a leading `-` when this value is negative.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`radix` is outside 2 to 36.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_bigint_of" => (nvs_core_bigint_of as *const ()).cast(),
        "nvs_core_bigint_of_uint" => (nvs_core_bigint_of_uint as *const ()).cast(),
        "nvs_core_bigint_parse" => (nvs_core_bigint_parse as *const ()).cast(),
        "nvs_core_bigint_add" => (nvs_core_bigint_add as *const ()).cast(),
        "nvs_core_bigint_sub" => (nvs_core_bigint_sub as *const ()).cast(),
        "nvs_core_bigint_mul" => (nvs_core_bigint_mul as *const ()).cast(),
        "nvs_core_bigint_div" => (nvs_core_bigint_div as *const ()).cast(),
        "nvs_core_bigint_mod" => (nvs_core_bigint_mod as *const ()).cast(),
        "nvs_core_bigint_pow" => (nvs_core_bigint_pow as *const ()).cast(),
        "nvs_core_bigint_pow_mod" => (nvs_core_bigint_pow_mod as *const ()).cast(),
        "nvs_core_bigint_sqrt" => (nvs_core_bigint_sqrt as *const ()).cast(),
        "nvs_core_bigint_gcd" => (nvs_core_bigint_gcd as *const ()).cast(),
        "nvs_core_bigint_lcm" => (nvs_core_bigint_lcm as *const ()).cast(),
        "nvs_core_bigint_neg" => (nvs_core_bigint_neg as *const ()).cast(),
        "nvs_core_bigint_abs" => (nvs_core_bigint_abs as *const ()).cast(),
        "nvs_core_bigint_shl" => (nvs_core_bigint_shl as *const ()).cast(),
        "nvs_core_bigint_shr" => (nvs_core_bigint_shr as *const ()).cast(),
        "nvs_core_bigint_sign" => (nvs_core_bigint_sign as *const ()).cast(),
        "nvs_core_bigint_compare_to" => (nvs_core_bigint_compare_to as *const ()).cast(),
        "nvs_core_bigint_to_int" => (nvs_core_bigint_to_int as *const ()).cast(),
        "nvs_core_bigint_to_uint" => (nvs_core_bigint_to_uint as *const ()).cast(),
        "nvs_core_bigint_to_decimal" => (nvs_core_bigint_to_decimal as *const ()).cast(),
        "nvs_core_bigint_to_string" => (nvs_core_bigint_to_string as *const ()).cast(),
        "nvs_core_bigint_format" => (nvs_core_bigint_format as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Shared reading and building
// ============================================================================

/// A fresh `BigInt` instance holding `value`, accounted against the request's
/// memory ceiling before the magnitude is copied into the heap.
///
/// # Errors
///
/// Whatever [`nvs_runtime::affordable`] answers for a request at its ceiling.
fn built(value: &BigInt) -> Result<Value, Fault> {
    let (sign, magnitude) = value.to_bytes_le();
    nvs_runtime::affordable(Some(magnitude.len()), NAME)?;
    Ok(crate::instance::build(
        &CLASS,
        [
            Value::int(match sign {
                Sign::Minus => -1,
                Sign::NoSign => 0,
                Sign::Plus => 1,
            }),
            Value::bytes(NvsStr::new(&magnitude)),
        ],
    ))
}

/// The `BigInt` in argument slot `at` — slot 0 for a receiver, any other slot
/// for a `BigInt` *parameter*.
///
/// Owned rather than borrowed: the two slots are an encoding rather than a
/// value, so every operand is materialized and this is the one place that
/// happens.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member where a slot holds something the row
/// above says it cannot, which compiled code cannot produce.
fn operand(args: &[Value], at: usize, member: &str) -> Result<BigInt, Fault> {
    let object = crate::instance::receiver(args[at], &CLASS, member)?;
    // Unreachable from source: both slots are written by [`built`] alone, and
    // no member mutates one, so a tag mismatch here is a paste error in this
    // module rather than anything a program can reach.
    let sign = crate::instance::slot(object, SIGN_SLOT)
        .as_int()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\BigInt::{member} found a non-`int` sign slot"
            ))
        })?;
    let magnitude = crate::instance::slot(object, MAGNITUDE_SLOT);
    // Unreachable from source, for the sign slot's reason.
    let bytes = magnitude.as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\BigInt::{member} found a non-`bytes` magnitude slot"
        ))
    })?;
    Ok(BigInt::from_bytes_le(
        match sign {
            ..=-1 => Sign::Minus,
            0 => Sign::NoSign,
            1.. => Sign::Plus,
        },
        bytes,
    ))
}

/// The `uint` in argument slot `at`, for a shift width or an exponent.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`operand`]'s reason.
fn width(args: &[Value], at: usize, member: &str) -> Result<u64, Fault> {
    // Unreachable from source: the row above declares a `CoreTy::Uint`, so a
    // value of another tag is `E0401` at the checker.
    args[at].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\BigInt::{member} expected a `uint`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// The `radix` option's value, checked against the 2-to-36 range both members
/// carrying [`RADIX_OPTIONS`] read and write.
///
/// # Errors
///
/// A `LogicError` for a radix outside that range — a bug in the program, not in
/// its input, which is why it is not the `ParseError` a bad digit is.
fn radix(value: &Value, member: &str) -> Result<u32, Fault> {
    // Unreachable from source: `radix` is a `CoreTy::Uint` option, so
    // `{radix: $r}` over anything else is `E0401` at the checker and an
    // omitting call site passes the `Const::Uint` default.
    let asked = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\BigInt::{member} expected a `uint` for the `radix` option, got tag {}",
            value.tag_byte()
        ))
    })?;
    if !(MIN_RADIX..=MAX_RADIX).contains(&asked) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\BigInt::{member}(): radix {asked} is outside the {MIN_RADIX} to \
                 {MAX_RADIX} this class reads and writes"
            ),
        ));
    }
    Ok(u32::try_from(asked).expect("a radix inside the checked range fits a `u32`"))
}

/// One `string` argument, as text — `parse`'s only one.
///
/// # Errors
///
/// A [`Fault::fatal`] for the wrong tag, which the row above rules out.
fn text_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    // Unreachable from source: the row declares a `CoreTy::Text`, so anything
    // else is `E0401` at the checker.
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\BigInt::{member} expected a `string`, got tag {}",
            value.tag_byte()
        ))
    })
}

/// `rule:types/arithmetic`'s refusal, at this class's magnitudes: the sentence
/// every member that cannot answer throws, as an `ArithmeticError`.
fn refused(message: String) -> Fault {
    Fault::thrown_as(ThrownClass::Arithmetic, message)
}

/// The three-way answer `sign` and `compareTo` share.
fn ordering(found: std::cmp::Ordering) -> Value {
    Value::int(match found {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

// ============================================================================
// Construction
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\BigInt::of(int $value): BigInt`.
    fn nvs_core_bigint_of(_ctx, args: [1]) {
        // Unreachable from source: the row declares a `CoreTy::Int`.
        let value = args[0].as_int().ok_or_else(|| {
            Fault::fatal(format!("Core\\BigInt::of expected an `int`, got tag {}", args[0].tag_byte()))
        })?;
        built(&BigInt::from(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\BigInt::ofUint(uint $value): BigInt` — the half of the unsigned
    /// range `of` cannot be handed.
    fn nvs_core_bigint_of_uint(_ctx, args: [1]) {
        built(&BigInt::from(width(args, 0, "ofUint")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\BigInt::parse(string $text, {radix?: uint}): BigInt`.
    ///
    /// What is accepted is an optional sign and then digits: no leading space,
    /// and no prefix, since `0x` is a radix written down rather than a spelling
    /// this member guesses at.
    ///
    /// **A `_` separator is refused**, which `BigInt::parse_bytes` would accept
    /// on its own. `1_000` is an integer *literal*'s spelling and not a
    /// `string`'s — `"1_000" as int` refuses it (`rule:types/conversion`) — and
    /// two spellings of "read this text as an integer" answering differently is
    /// the divergence worth one comparison per call to avoid.
    fn nvs_core_bigint_parse(_ctx, args: [2]) {
        let text = text_of(&args[0], "parse")?;
        let radix = radix(&args[1], "parse")?;
        Some(text)
            .filter(|text| !text.as_bytes().contains(&b'_'))
            .and_then(|text| BigInt::parse_bytes(text.as_bytes(), radix))
            .ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Parse,
                    format!("Core\\BigInt::parse(): `{text}` is not an integer in radix {radix}"),
                )
            })
            .and_then(|value| built(&value))
    }
}

// ============================================================================
// Arithmetic — each answering a new value, since every `Core` member is pure
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `$n->add(BigInt $other): BigInt`.
    fn nvs_core_bigint_add(_ctx, args: [2]) {
        built(&(operand(args, 0, "add")? + operand(args, 1, "add")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->sub(BigInt $other): BigInt`.
    fn nvs_core_bigint_sub(_ctx, args: [2]) {
        built(&(operand(args, 0, "sub")? - operand(args, 1, "sub")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->mul(BigInt $other): BigInt`.
    fn nvs_core_bigint_mul(_ctx, args: [2]) {
        built(&(operand(args, 0, "mul")? * operand(args, 1, "mul")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->div(BigInt $divisor): BigInt`, truncating toward zero — `intdiv`'s
    /// rounding, which is Rust's `/` on integers and not a rule this module
    /// implements.
    fn nvs_core_bigint_div(_ctx, args: [2]) {
        let dividend = operand(args, 0, "div")?;
        let divisor = operand(args, 1, "div")?;
        if divisor.sign() == Sign::NoSign {
            return Err(refused("Core\\BigInt::div(): the divisor is zero".to_owned()));
        }
        built(&(dividend / divisor))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->mod(BigInt $divisor): BigInt`, carrying the dividend's sign — `%`
    /// on `int`, which is Rust's `%` and not a rule this module implements.
    /// `Core\Math::mod`'s Euclidean answer is a different member of a different
    /// class and stays there.
    fn nvs_core_bigint_mod(_ctx, args: [2]) {
        let dividend = operand(args, 0, "mod")?;
        let divisor = operand(args, 1, "mod")?;
        if divisor.sign() == Sign::NoSign {
            return Err(refused("Core\\BigInt::mod(): the divisor is zero".to_owned()));
        }
        built(&(dividend % divisor))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->pow(uint $exponent): BigInt`, refused ahead of the work where the
    /// answer would be wider than [`MAX_BITS`] — see this module's
    /// § *Decision: one call may not explode*.
    fn nvs_core_bigint_pow(_ctx, args: [2]) {
        let base = operand(args, 0, "pow")?;
        let exponent = width(args, 1, "pow")?;
        let bits = base.bits().saturating_mul(exponent);
        let exponent = u32::try_from(exponent).unwrap_or(u32::MAX);
        if bits > MAX_BITS {
            return Err(refused(format!(
                "Core\\BigInt::pow(): the answer would be {bits} bits wide, past the \
                 {MAX_BITS} one call may produce"
            )));
        }
        built(&base.pow(exponent))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->powMod(BigInt $exponent, BigInt $modulus): BigInt` — `bcpowmod`,
    /// computed by `modpow` so the whole power is never held.
    ///
    /// The two refusals are `modpow`'s own panics, turned into throws ahead of
    /// the call: it panics on a zero modulus, and on a negative exponent where
    /// the receiver has no inverse.
    fn nvs_core_bigint_pow_mod(_ctx, args: [3]) {
        let base = operand(args, 0, "powMod")?;
        let exponent = operand(args, 1, "powMod")?;
        let modulus = operand(args, 2, "powMod")?;
        if modulus.sign() == Sign::NoSign {
            return Err(refused("Core\\BigInt::powMod(): the modulus is zero".to_owned()));
        }
        if exponent.sign() == Sign::Minus {
            return Err(refused(
                "Core\\BigInt::powMod(): the exponent is negative".to_owned(),
            ));
        }
        built(&base.modpow(&exponent, &modulus))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->sqrt(): BigInt`, the floor — `sqrt` panics below zero, so the
    /// refusal is written ahead of it.
    fn nvs_core_bigint_sqrt(_ctx, args: [1]) {
        let value = operand(args, 0, "sqrt")?;
        if value.sign() == Sign::Minus {
            return Err(refused(
                "Core\\BigInt::sqrt(): a negative value has no integer square root".to_owned(),
            ));
        }
        built(&value.sqrt())
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->gcd(BigInt $other): BigInt`, never negative.
    fn nvs_core_bigint_gcd(_ctx, args: [2]) {
        built(&operand(args, 0, "gcd")?.gcd(&operand(args, 1, "gcd")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->lcm(BigInt $other): BigInt`, never negative.
    ///
    /// Two zeroes answer zero here rather than reaching `lcm`, whose quotient
    /// by the gcd would be a division by zero.
    fn nvs_core_bigint_lcm(_ctx, args: [2]) {
        let left = operand(args, 0, "lcm")?;
        let right = operand(args, 1, "lcm")?;
        if left.sign() == Sign::NoSign && right.sign() == Sign::NoSign {
            return built(&BigInt::from(0));
        }
        built(&left.lcm(&right))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->neg(): BigInt`.
    fn nvs_core_bigint_neg(_ctx, args: [1]) {
        built(&-operand(args, 0, "neg")?)
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->abs(): BigInt` — total, unlike `abs` on `int`, whose most negative
    /// value has no positive twin.
    fn nvs_core_bigint_abs(_ctx, args: [1]) {
        // `num_traits::Signed::abs` is the crate's own spelling, and reaching it
        // would put a third `num-*` crate in this graph for one negation.
        let value = operand(args, 0, "abs")?;
        built(&if value.sign() == Sign::Minus { -value } else { value })
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->shl(uint $bits): BigInt`, refused ahead of the work at
    /// [`MAX_BITS`] for `pow`'s reason.
    fn nvs_core_bigint_shl(_ctx, args: [2]) {
        let value = operand(args, 0, "shl")?;
        let bits = width(args, 1, "shl")?;
        let wide = value.bits().saturating_add(bits);
        if wide > MAX_BITS {
            return Err(refused(format!(
                "Core\\BigInt::shl(): the answer would be {wide} bits wide, past the \
                 {MAX_BITS} one call may produce"
            )));
        }
        built(&(value << bits))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->shr(uint $bits): BigInt`, arithmetic — a negative value shifted
    /// past its own width answers `-1` rather than `0`, which is what rounding
    /// toward negative infinity means here.
    fn nvs_core_bigint_shr(_ctx, args: [2]) {
        let value = operand(args, 0, "shr")?;
        let bits = width(args, 1, "shr")?;
        built(&(value >> bits))
    }
}

// ============================================================================
// Reading a value back out
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `$n->sign(): int`.
    fn nvs_core_bigint_sign(_ctx, args: [1]) {
        Ok(Value::int(match operand(args, 0, "sign")?.sign() {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->compareTo(BigInt $other): int` — `Comparable`'s member
    /// (`rule:classes/comparable`), which is also what `<=>` and
    /// `Core\Arr::sort` reach through this class's descriptor.
    fn nvs_core_bigint_compare_to(_ctx, args: [2]) {
        let left = operand(args, 0, "compareTo")?;
        let right = operand(args, 1, "compareTo")?;
        Ok(ordering(left.cmp(&right)))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->toInt(): int`, exactly or not at all.
    fn nvs_core_bigint_to_int(_ctx, args: [1]) {
        let value = operand(args, 0, "toInt")?;
        i64::try_from(&value)
            .map(Value::int)
            .map_err(|_| refused(format!("Core\\BigInt::toInt(): {value} is outside the `int` range")))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->toUint(): uint`, exactly or not at all — every negative value is
    /// outside the range, which is the half `toInt` accepts and this does not.
    fn nvs_core_bigint_to_uint(_ctx, args: [1]) {
        let value = operand(args, 0, "toUint")?;
        u64::try_from(&value)
            .map(Value::uint)
            .map_err(|_| refused(format!("Core\\BigInt::toUint(): {value} is outside the `uint` range")))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->toDecimal(): decimal` at scale zero — exact up to the scalar's
    /// 96-bit mantissa, and refused above it rather than rounded, since a
    /// `decimal` is the exact scalar (`rule:types/decimal`).
    fn nvs_core_bigint_to_decimal(_ctx, args: [1]) {
        let value = operand(args, 0, "toDecimal")?;
        let outside =
            || refused(format!("Core\\BigInt::toDecimal(): {value} is wider than a `decimal` holds"));
        let magnitude = u128::try_from(value.magnitude()).map_err(|_| outside())?;
        Decimal::new(value.sign() == Sign::Minus, magnitude, 0)
            .map(Value::decimal)
            .ok_or_else(outside)
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->toString(): string` in radix 10 — `Stringable`'s member
    /// (`rule:classes/stringable`), which is what `echo` reaches through this
    /// class's descriptor.
    fn nvs_core_bigint_to_string(_ctx, args: [1]) {
        let text = operand(args, 0, "toString")?.to_string();
        nvs_runtime::affordable(Some(text.len()), NAME)?;
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `$n->format({radix?: uint}): string` — `toString` in any base `parse`
    /// reads back.
    fn nvs_core_bigint_format(_ctx, args: [2]) {
        let value = operand(args, 0, "format")?;
        let text = value.to_str_radix(radix(&args[1], "format")?);
        nvs_runtime::affordable(Some(text.len()), NAME)?;
        Ok(Value::str(NvsStr::new(text.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, call};

    use super::*;

    /// One instance holding `value`, built exactly as every member's answer is.
    ///
    /// Each one is handed to a single [`call`], which consumes the reference it
    /// was given, so every fixture here is built for one use and the answers are
    /// left to the process — a crate under the workspace's `unsafe_code` lint
    /// has nothing to release a [`Value`] with.
    fn of(value: i64) -> Value {
        built(&BigInt::from(value)).expect("a one-word magnitude is affordable")
    }

    /// The magnitude a member answered with, read back out of its two slots.
    fn read(answer: Value) -> BigInt {
        operand(&[answer], 0, "toString").expect("a member answers with its own class")
    }

    /// `div` truncates toward zero and `mod` carries the dividend's sign — the
    /// two roundings `intdiv` and `%` have on `int`, asserted by **counting**
    /// over a sign sweep rather than read off a line, so a member that grew a
    /// rounding of its own fails here while still answering plausibly on the
    /// positive row everyone reads first.
    // covers: Core\BigInt::div, Core\BigInt::mod
    #[test]
    fn bigint_div_and_mod_agree_over_a_sign_sweep() {
        const SWEEP: [(i64, i64); 12] = [
            (7, 2),
            (7, -2),
            (-7, 2),
            (-7, -2),
            (9, 3),
            (9, -3),
            (-9, 3),
            (-9, -3),
            (1, 7),
            (-1, 7),
            (0, 5),
            (0, -5),
        ];

        let mut ctx = Ctx::buffered();
        let mut agreed = 0usize;
        for (dividend, divisor) in SWEEP {
            let quotient = call(nvs_core_bigint_div, &mut ctx, &[of(dividend), of(divisor)])
                .expect("a non-zero divisor answers");
            let remainder = call(nvs_core_bigint_mod, &mut ctx, &[of(dividend), of(divisor)])
                .expect("a non-zero divisor answers");
            if read(quotient) == BigInt::from(dividend / divisor) {
                agreed += 1;
            }
            if read(remainder) == BigInt::from(dividend % divisor) {
                agreed += 1;
            }
        }
        assert_eq!(agreed, SWEEP.len() * 2);
    }

    /// Both sides of the bound `toInt` is written around, named together: the
    /// last value an `int` holds comes back, and the first one it does not is
    /// refused as an `ArithmeticError` rather than truncated — a member that
    /// stopped one entry early prints plausibly against either half alone.
    #[test]
    fn a_bigint_past_the_int_range_refuses_to_int() {
        let mut ctx = Ctx::buffered();

        let held = call(nvs_core_bigint_to_int, &mut ctx, &[of(i64::MAX)])
            .expect("the last value an `int` holds narrows");
        assert_eq!(held.as_int(), Some(i64::MAX));

        let past = built(&(BigInt::from(i64::MAX) + BigInt::from(1)))
            .expect("a two-word magnitude is affordable");
        call(nvs_core_bigint_to_int, &mut ctx, &[past]).expect_err("one past it is refused");
        assert_eq!(
            ctx.take_pending().map(std::borrow::Cow::into_owned),
            Some(
                "Core\\BigInt::toInt(): 9223372036854775808 is outside the `int` range".to_owned()
            )
        );
    }

    /// `compareTo` answers one of exactly three values, reverses when its
    /// operands do, and agrees with the order of the magnitudes behind them —
    /// counted over every pair of a table rather than read off a line, so a pair
    /// that happened to answer plausibly cannot carry the row. The two entries
    /// that differ in their lowest bit alone are what a comparison stopping at
    /// the first word would call equal.
    // covers: Core\BigInt::compareTo
    #[test]
    fn compare_to_is_three_valued_and_reverses_with_its_operands() {
        let table = [
            BigInt::from(0),
            BigInt::from(-1),
            BigInt::from(1),
            BigInt::from(i64::MIN),
            BigInt::from(i64::MAX),
            (BigInt::from(1) << 300) - BigInt::from(1),
            BigInt::from(1) << 300,
        ];

        let mut ctx = Ctx::buffered();
        let mut pairs = 0usize;
        let mut three_valued = 0usize;
        let mut reversed = 0usize;
        let mut ordered = 0usize;
        for left in &table {
            for right in &table {
                pairs += 1;
                let forward = call(
                    nvs_core_bigint_compare_to,
                    &mut ctx,
                    &[
                        built(left).expect("a table magnitude is affordable"),
                        built(right).expect("a table magnitude is affordable"),
                    ],
                )
                .expect("every pair orders")
                .as_int()
                .expect("the answer is an `int`");
                let backward = call(
                    nvs_core_bigint_compare_to,
                    &mut ctx,
                    &[
                        built(right).expect("a table magnitude is affordable"),
                        built(left).expect("a table magnitude is affordable"),
                    ],
                )
                .expect("either order orders")
                .as_int()
                .expect("the answer is an `int`");
                if (-1..=1).contains(&forward) {
                    three_valued += 1;
                }
                if forward == -backward {
                    reversed += 1;
                }
                if forward == ordering(left.cmp(right)).as_int().expect("an `int`") {
                    ordered += 1;
                }
            }
        }
        assert_eq!(pairs, table.len() * table.len());
        assert_eq!((three_valued, reversed, ordered), (pairs, pairs, pairs));
    }

    /// `add` answers the same sum in either order and grows one by at most a
    /// single bit past its wider operand — the bound this module's § *Decision:
    /// one call may not explode* rests on when it leaves `add` unchecked, and
    /// the only place it is asserted. Counted over a sign sweep that includes
    /// both ends of the `int` range, so a pair that happened to answer plausibly
    /// cannot carry the row on its own.
    // covers: Core\BigInt::add
    #[test]
    fn add_is_commutative_and_grows_by_one_bit_at_most() {
        const SWEEP: [(i64, i64); 9] = [
            (0, 0),
            (7, 5),
            (-7, 5),
            (7, -5),
            (-7, -5),
            (i64::MAX, i64::MAX),
            (i64::MIN, i64::MIN),
            (i64::MAX, i64::MIN),
            (i64::MIN, 1),
        ];

        let mut ctx = Ctx::buffered();
        let mut summed = 0usize;
        let mut commuted = 0usize;
        let mut bounded = 0usize;
        for (left, right) in SWEEP {
            let forward = read(
                call(nvs_core_bigint_add, &mut ctx, &[of(left), of(right)])
                    .expect("every pair sums"),
            );
            let backward = read(
                call(nvs_core_bigint_add, &mut ctx, &[of(right), of(left)])
                    .expect("either order sums"),
            );
            if forward == BigInt::from(left) + BigInt::from(right) {
                summed += 1;
            }
            if backward == forward {
                commuted += 1;
            }
            let widest = BigInt::from(left).bits().max(BigInt::from(right).bits());
            if forward.bits() <= widest + 1 {
                bounded += 1;
            }
        }
        assert_eq!(
            (summed, commuted, bounded),
            (SWEEP.len(), SWEEP.len(), SWEEP.len())
        );
    }

    /// `mul` answers the same product in either order and is as wide as its two
    /// operands together, give or take the one bit a carry moves — the shape
    /// every wide multiplication has, and the property that fails first when a
    /// product is truncated. Counted over a sign sweep that includes both ends
    /// of the `int` range. Zero is named on its own afterwards, because it is
    /// the one pair whose product is narrower than either operand.
    // covers: Core\BigInt::mul
    #[test]
    fn mul_is_commutative_and_as_wide_as_its_operands_together() {
        const SWEEP: [(i64, i64); 8] = [
            (0, 0),
            (7, 5),
            (-7, 5),
            (7, -5),
            (-7, -5),
            (i64::MAX, i64::MAX),
            (i64::MIN, i64::MIN),
            (1, i64::MIN),
        ];

        let mut ctx = Ctx::buffered();
        let mut multiplied = 0usize;
        let mut commuted = 0usize;
        let mut as_wide = 0usize;
        for (left, right) in SWEEP {
            let forward = read(
                call(nvs_core_bigint_mul, &mut ctx, &[of(left), of(right)])
                    .expect("every pair multiplies"),
            );
            let backward = read(
                call(nvs_core_bigint_mul, &mut ctx, &[of(right), of(left)])
                    .expect("either order multiplies"),
            );
            if forward == BigInt::from(left) * BigInt::from(right) {
                multiplied += 1;
            }
            if backward == forward {
                commuted += 1;
            }
            let together = BigInt::from(left).bits() + BigInt::from(right).bits();
            if forward.bits() == together || forward.bits() + 1 == together {
                as_wide += 1;
            }
        }
        assert_eq!(
            (multiplied, commuted, as_wide),
            (SWEEP.len(), SWEEP.len(), SWEEP.len())
        );

        let absorbed = read(
            call(nvs_core_bigint_mul, &mut ctx, &[of(0), of(i64::MAX)])
                .expect("zero multiplies too"),
        );
        assert_eq!(absorbed, BigInt::from(0));
    }

    /// `abs` never answers a negative, keeps the magnitude it was handed and
    /// agrees with itself when applied twice — asserted by **counting** over a
    /// sign sweep rather than read off a line, so a member that answered
    /// plausibly on the row everyone checks first still fails here. The floor of
    /// the `int` range is named on its own afterwards: it is the one value
    /// `Core\Math::abs` refuses, and this member is total there.
    // covers: Core\BigInt::abs
    #[test]
    fn abs_is_never_negative_and_is_total_at_the_int_floor() {
        const SWEEP: [i64; 7] = [i64::MIN, -9, -1, 0, 1, 9, i64::MAX];

        let mut ctx = Ctx::buffered();
        let mut non_negative = 0usize;
        let mut same_magnitude = 0usize;
        let mut idempotent = 0usize;
        for value in SWEEP {
            let once = read(
                call(nvs_core_bigint_abs, &mut ctx, &[of(value)]).expect("every value has one"),
            );
            let again = read(
                call(
                    nvs_core_bigint_abs,
                    &mut ctx,
                    &[built(&once).expect("a one-word magnitude is affordable")],
                )
                .expect("a magnitude has one too"),
            );
            let held = BigInt::from(value);
            let flipped = -held.clone();
            if once.sign() != Sign::Minus {
                non_negative += 1;
            }
            if once == held || once == flipped {
                same_magnitude += 1;
            }
            if again == once {
                idempotent += 1;
            }
        }
        assert_eq!(
            (non_negative, same_magnitude, idempotent),
            (SWEEP.len(), SWEEP.len(), SWEEP.len())
        );

        let floor = read(
            call(nvs_core_bigint_abs, &mut ctx, &[of(i64::MIN)])
                .expect("the smallest `int` has a magnitude here"),
        );
        assert_eq!(floor.to_string(), "9223372036854775808");
    }

    /// `format` writes a value in every radix this class allows, using only the
    /// digits that radix has and one leading `-` for a negative — counted over
    /// the whole range rather than read off a line, so a base whose digit class
    /// is wrong in one direction only fails here while 10 and 16 go on printing
    /// plausibly. Radix 10 is `toString`'s own text, which is the row a default
    /// that drifted would break. Both sides of the range are named afterwards:
    /// the widest radix writes, and the first one past it is a `LogicError`
    /// rather than digits nothing can read back.
    // covers: Core\BigInt::format
    #[test]
    fn format_writes_every_radix_it_allows_and_refuses_both_sides_of_the_range() {
        /// The digits in order, of which a radix uses the first `radix` many.
        const DIGITS: &str = "0123456789abcdefghijklmnopqrstuvwxyz";
        const VALUE: i64 = -123_456_789_012_345;

        let mut ctx = Ctx::buffered();
        let mut agreed = 0usize;
        let mut negative = 0usize;
        let mut in_alphabet = 0usize;
        for asked in MIN_RADIX..=MAX_RADIX {
            let answer = call(
                nvs_core_bigint_format,
                &mut ctx,
                &[of(VALUE), Value::uint(asked)],
            )
            .expect("every radix inside the range writes");
            let written = answer.as_text().expect("format answers text");
            let narrow = u32::try_from(asked).expect("a radix fits a `u32`");
            if written == BigInt::from(VALUE).to_str_radix(narrow) {
                agreed += 1;
            }
            let digits = written.strip_prefix('-').unwrap_or(written);
            if written.starts_with('-') && written.matches('-').count() == 1 {
                negative += 1;
            }
            let alphabet = &DIGITS[..narrow as usize];
            if digits.chars().all(|digit| alphabet.contains(digit)) {
                in_alphabet += 1;
            }
        }
        let radixes = (MIN_RADIX..=MAX_RADIX).count();
        assert_eq!((agreed, negative, in_alphabet), (radixes, radixes, radixes));

        let decimal = call(
            nvs_core_bigint_format,
            &mut ctx,
            &[of(VALUE), Value::uint(DEFAULT_RADIX)],
        )
        .expect("the default radix writes");
        let rendered =
            call(nvs_core_bigint_to_string, &mut ctx, &[of(VALUE)]).expect("every value renders");
        assert_eq!(decimal.as_text(), rendered.as_text());

        call(
            nvs_core_bigint_format,
            &mut ctx,
            &[of(VALUE), Value::uint(MAX_RADIX + 1)],
        )
        .expect_err("one radix past the range is refused");
        assert_eq!(
            ctx.take_pending().map(std::borrow::Cow::into_owned),
            Some(
                "Core\\BigInt::format(): radix 37 is outside the 2 to 36 this class reads and \
                 writes"
                    .to_owned()
            )
        );
        call(
            nvs_core_bigint_format,
            &mut ctx,
            &[of(VALUE), Value::uint(MIN_RADIX - 1)],
        )
        .expect_err("one radix below the range is refused too");
    }

    /// `gcd` and `lcm` are one identity rather than two tables of remembered
    /// answers: their product is the magnitude of the operands' product, the
    /// divisor divides both operands exactly, and neither member ever answers a
    /// negative. Counted over a sign sweep that includes a zero operand and the
    /// floor of the `int` range, so a member that is right on the positive row
    /// everyone reads first still fails here. Two zeroes are named afterwards:
    /// they are the pair whose divisor is zero, which the identity's own
    /// quotient cannot be written over.
    // covers: Core\BigInt::gcd, Core\BigInt::lcm
    #[test]
    fn gcd_and_lcm_multiply_to_the_magnitude_of_their_operands_product() {
        const SWEEP: [(i64, i64); 10] = [
            (0, 5),
            (5, 0),
            (1, 1),
            (6, 4),
            (-6, 4),
            (6, -4),
            (-6, -4),
            (360, 48),
            (17, 97),
            (i64::MIN, 6),
        ];

        let mut ctx = Ctx::buffered();
        let mut multiplied = 0usize;
        let mut divides = 0usize;
        let mut non_negative = 0usize;
        for (left, right) in SWEEP {
            let divisor = read(
                call(nvs_core_bigint_gcd, &mut ctx, &[of(left), of(right)])
                    .expect("every pair has a divisor"),
            );
            let multiple = read(
                call(nvs_core_bigint_lcm, &mut ctx, &[of(left), of(right)])
                    .expect("every pair has a multiple"),
            );
            let product = BigInt::from(left) * BigInt::from(right);
            let magnitude = if product.sign() == Sign::Minus {
                -product
            } else {
                product
            };
            if &divisor * &multiple == magnitude {
                multiplied += 1;
            }
            if divisor.sign() != Sign::Minus && multiple.sign() != Sign::Minus {
                non_negative += 1;
            }
            if divisor.sign() == Sign::NoSign
                || (BigInt::from(left) % &divisor == BigInt::from(0)
                    && BigInt::from(right) % &divisor == BigInt::from(0))
            {
                divides += 1;
            }
        }
        assert_eq!(
            (multiplied, divides, non_negative),
            (SWEEP.len(), SWEEP.len(), SWEEP.len())
        );

        let divisor = read(
            call(nvs_core_bigint_gcd, &mut ctx, &[of(0), of(0)]).expect("two zeroes answer here"),
        );
        let multiple = read(
            call(nvs_core_bigint_lcm, &mut ctx, &[of(0), of(0)]).expect("and answer here too"),
        );
        assert_eq!((divisor, multiple), (BigInt::from(0), BigInt::from(0)));
    }

    /// The two members the engine reaches by name rather than through the
    /// method table — this module's § *Decision: a registered member the engine
    /// reaches by name gets a descriptor field*. Both halves are asserted: the
    /// rosters [`crate::instance::descriptors`] derives the descriptor's
    /// `renderer` and `comparer` from name this class and these symbols, and
    /// [`address`] resolves each, which is the arm whose miss is a runtime
    /// panic rather than a link error.
    #[test]
    fn a_bigint_renders_and_compares_through_its_descriptor() {
        assert!(crate::registry::class_renders(NAME));
        assert!(crate::registry::implements_comparable(NAME));
        assert_eq!(
            crate::registry::render_symbol(NAME),
            Some("nvs_core_bigint_to_string")
        );
        assert_eq!(
            crate::registry::compare_symbol(NAME),
            Some("nvs_core_bigint_compare_to")
        );
        assert!(address("nvs_core_bigint_to_string").is_some());
        assert!(address("nvs_core_bigint_compare_to").is_some());

        let mut ctx = Ctx::buffered();
        let rendered =
            call(nvs_core_bigint_to_string, &mut ctx, &[of(-42)]).expect("every value renders");
        assert_eq!(rendered.as_text(), Some("-42"));
        let order = call(nvs_core_bigint_compare_to, &mut ctx, &[of(2), of(10)])
            .expect("every pair compares");
        assert_eq!(order.as_int(), Some(-1));
    }
}
