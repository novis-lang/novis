//! `Core\Math` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 3, over `int`, `uint` and `float`.
//!
//! `**` is exponentiation and `%` is integer modulo, so neither has a member
//! (`rule:core-api/shape-rules` R17); that section's own note owns why.
//!
//! # Two rules cover every member here, so no member restates them
//!
//! * **A division by zero throws `ArithmeticError`**, in `intDiv` and in `mod`
//!   alike, and in the `/` operator over every operand type (`rule:types/arithmetic`) —
//!   spec § 3's opening paragraph. [`nvs_core_math_fdiv`] is the single
//!   exception and is there to be one: with the operator throwing on a `float`
//!   divisor too, it is the only way left to ask for IEEE's infinity, which is
//!   why PHP has the same function for the same reason. So does an operation
//!   whose exact answer does
//!   not fit its result type: `intDiv(int::MIN, -1)`, `abs(int::MIN)`, an
//!   `lcm` past `int`, a `uint` argument past `int` — `rule:types/arithmetic`'s class for
//!   an overflow, the same one the operators raise. A refusal that is about
//!   the *argument* rather than the arithmetic — a base outside 2..=36, an
//!   empty range for `clamp`, a `NaN` for `sign` — stays a `RuntimeError`.
//! * **A domain error is IEEE's answer, not a throw.** `sqrt(-1.0)`,
//!   `log(0.0)`, `asin(2.0)` and the rest produce `NaN` or an infinity exactly
//!   as PHP's do, and [`nvs_core_math_is_nan`]/[`nvs_core_math_is_finite`] are
//!   the members that ask. This is priority 2 — PHP-compatible observable
//!   behaviour — and it is the reason those two members exist at all; a
//!   library that threw instead would leave them with nothing to answer about.
//!   A *division* is not a domain error, which is what keeps the two rules
//!   from meeting — and [`nvs_core_math_fdiv`] does not blur them either, being
//!   a member whose whole name is the IEEE answer rather than a division that
//!   changed its mind.
//!
//! # The four rounding members are `float`'s, and `decimal` has its own
//!
//! The spec writes `int|float|decimal` at seven rows. Three of them — `abs`,
//! `sign` and `format` — take it: [`NUMBER`] is that union, and [`Number`] is
//! how a member reads one back. The other four are `ceil`, `floor`,
//! `truncate` and `round`, and they are registered at `float` alone because
//! widening them would be a change of *result* type rather than one more
//! decode arm: an exact rounding has to answer a `decimal` to be worth
//! anything, and a `float` answer would throw away the very scale the cut was
//! asked for. So `Core\Decimal` carries its own four
//! (`rule:types/conversion`), answering a `decimal` at the scale named, and
//! [`round_mode`] is the one reader both classes' `mode` argument goes
//! through.

use nvs_runtime::{Ctx, Decimal, Fault, NvsStr, Tag, ThrownClass, Value};

use crate::ordering::compare_values;
use crate::registry::{
    CaseDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Math`'s registry rows, in the spec's own order.
///
/// Declared beside the implementations rather than in one flat table, for the
/// reason [`crate::arr::CLASS`] states.
pub const CLASS: CoreClass = CoreClass {
    name: r"Core\Math",
    doc: None,
    methods: &[
        CoreMethod {
            name: "abs",
            names: &["n"],
            params: &[CoreTy::Union(NUMBER)],
            defaults: &[],
            return_ty: CoreTy::Union(NUMBER),
            symbol: "nvs_core_math_abs",
            doc: Some(&ABS_DOC),
        },
        CoreMethod {
            name: "sign",
            names: &["n"],
            params: &[CoreTy::Union(NUMBER)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_math_sign",
            doc: Some(&SIGN_DOC),
        },
        CoreMethod {
            name: "min",
            names: &["a", "b"],
            params: &[CoreTy::Var("T"), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_math_min",
            doc: Some(&MIN_DOC),
        },
        CoreMethod {
            name: "max",
            names: &["a", "b"],
            params: &[CoreTy::Var("T"), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_math_max",
            doc: Some(&MAX_DOC),
        },
        CoreMethod {
            name: "clamp",
            names: &["n", "low", "high"],
            params: &[CoreTy::Var("T"), CoreTy::Var("T"), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_math_clamp",
            doc: Some(&CLAMP_DOC),
        },
        CoreMethod {
            name: "ceil",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_ceil",
            doc: Some(&CEIL_DOC),
        },
        CoreMethod {
            name: "floor",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_floor",
            doc: Some(&FLOOR_DOC),
        },
        CoreMethod {
            name: "truncate",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_truncate",
            doc: Some(&TRUNCATE_DOC),
        },
        CoreMethod {
            name: "round",
            names: &["n"],
            params: &[CoreTy::Float, CoreTy::Options(ROUND_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_round",
            doc: Some(&ROUND_DOC),
        },
        CoreMethod {
            name: "intDiv",
            names: &["a", "b"],
            params: &[CoreTy::Int, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_math_int_div",
            doc: Some(&INT_DIV_DOC),
        },
        CoreMethod {
            name: "mod",
            names: &["a", "b"],
            params: &[CoreTy::Float, CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_mod",
            doc: Some(&MOD_DOC),
        },
        CoreMethod {
            name: "fdiv",
            names: &["a", "b"],
            params: &[CoreTy::Float, CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_fdiv",
            doc: Some(&FDIV_DOC),
        },
        CoreMethod {
            name: "gcd",
            names: &["a", "b"],
            params: &[CoreTy::Int, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_math_gcd",
            doc: Some(&GCD_DOC),
        },
        CoreMethod {
            name: "lcm",
            names: &["a", "b"],
            params: &[CoreTy::Int, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_math_lcm",
            doc: Some(&LCM_DOC),
        },
        CoreMethod {
            name: "sqrt",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_sqrt",
            doc: Some(&SQRT_DOC),
        },
        CoreMethod {
            name: "cbrt",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_cbrt",
            doc: Some(&CBRT_DOC),
        },
        CoreMethod {
            name: "hypot",
            names: &["a", "b"],
            params: &[CoreTy::Float, CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_hypot",
            doc: Some(&HYPOT_DOC),
        },
        CoreMethod {
            name: "exp",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_exp",
            doc: Some(&EXP_DOC),
        },
        CoreMethod {
            name: "log",
            names: &["n"],
            params: &[CoreTy::Float, CoreTy::Options(LOG_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_log",
            doc: Some(&LOG_DOC),
        },
        CoreMethod {
            name: "sin",
            names: &["radians"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_sin",
            doc: Some(&SIN_DOC),
        },
        CoreMethod {
            name: "cos",
            names: &["radians"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_cos",
            doc: Some(&COS_DOC),
        },
        CoreMethod {
            name: "tan",
            names: &["radians"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_tan",
            doc: Some(&TAN_DOC),
        },
        CoreMethod {
            name: "asin",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_asin",
            doc: Some(&ASIN_DOC),
        },
        CoreMethod {
            name: "acos",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_acos",
            doc: Some(&ACOS_DOC),
        },
        CoreMethod {
            name: "atan",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_atan",
            doc: Some(&ATAN_DOC),
        },
        CoreMethod {
            name: "atan2",
            names: &["y", "x"],
            params: &[CoreTy::Float, CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_atan2",
            doc: Some(&ATAN2_DOC),
        },
        CoreMethod {
            name: "sinh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_sinh",
            doc: Some(&SINH_DOC),
        },
        CoreMethod {
            name: "cosh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_cosh",
            doc: Some(&COSH_DOC),
        },
        CoreMethod {
            name: "tanh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_tanh",
            doc: Some(&TANH_DOC),
        },
        CoreMethod {
            name: "asinh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_asinh",
            doc: Some(&ASINH_DOC),
        },
        CoreMethod {
            name: "acosh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_acosh",
            doc: Some(&ACOSH_DOC),
        },
        CoreMethod {
            name: "atanh",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_atanh",
            doc: Some(&ATANH_DOC),
        },
        CoreMethod {
            name: "toRadians",
            names: &["degrees"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_to_radians",
            doc: Some(&TO_RADIANS_DOC),
        },
        CoreMethod {
            name: "toDegrees",
            names: &["radians"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_math_to_degrees",
            doc: Some(&TO_DEGREES_DOC),
        },
        CoreMethod {
            name: "isNan",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_math_is_nan",
            doc: Some(&IS_NAN_DOC),
        },
        CoreMethod {
            name: "isFinite",
            names: &["n"],
            params: &[CoreTy::Float],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_math_is_finite",
            doc: Some(&IS_FINITE_DOC),
        },
        CoreMethod {
            name: "toBase",
            names: &["n", "base"],
            params: &[CoreTy::Int, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_math_to_base",
            doc: Some(&TO_BASE_DOC),
        },
        CoreMethod {
            name: "fromBase",
            names: &["s", "base"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_math_from_base",
            doc: Some(&FROM_BASE_DOC),
        },
        CoreMethod {
            name: "format",
            names: &["n"],
            params: &[CoreTy::Union(NUMBER), CoreTy::Options(FORMAT_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_math_format",
            doc: Some(&FORMAT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: CONSTANTS,
};

/// `Core\Math::abs`'s reference card — `rule:core-api/reference-card`.
const ABS_DOC: MethodDoc = MethodDoc {
    short: "The magnitude of `$n`, in `$n`'s own type, as `abs` does — the one member here whose \
            result type is the argument's.",
    params: &[ParamDoc {
        name: "n",
        desc: "The number to take the magnitude of: an `int`, a `float` or a `decimal`.",
        shape: &[],
    }],
    ret: "`$n` with its sign dropped, in the type it came in; `-0.0` becomes `0.0`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When `$n` is `INT_MIN`, whose magnitude is one past `INT_MAX`, or a `uint` past \
               `INT_MAX`.",
    }],
};

/// `Core\Math::sign`'s reference card — `rule:core-api/reference-card`.
const SIGN_DOC: MethodDoc = MethodDoc {
    short: "Which side of zero `$n` is on — `-1`, `0` or `1` — as PHP's `$n <=> 0` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The number to classify: an `int`, a `float` or a `decimal`.",
        shape: &[],
    }],
    ret: "`-1` below zero, `1` above it and `0` for zero, `-0.0` included.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "When `$n` is `NaN`, which is on neither side of zero.",
        },
        ErrorDoc {
            error: "ArithmeticError",
            desc: "When `$n` is a `uint` past `INT_MAX`.",
        },
    ],
};

/// `Core\Math::min`'s reference card — `rule:core-api/reference-card`.
const MIN_DOC: MethodDoc = MethodDoc {
    short: "The smaller of two values under their natural order, as `min` does with two scalar \
            arguments; the array form is `Core\\Arr::min`.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One value, of a type with a natural order: a number, a string, a `bool` or \
                   `null`.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other value, of the same type as `$a`.",
            shape: &[],
        },
    ],
    ret: "Whichever compares smaller; `$a` on a tie, where PHP's `min` answers `$b`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When the pair has no natural order — an object, an array, or two values of \
               different kinds.",
    }],
};

/// `Core\Math::max`'s reference card — `rule:core-api/reference-card`.
const MAX_DOC: MethodDoc = MethodDoc {
    short: "The larger of two values under their natural order, as `max` does with two scalar \
            arguments; the array form is `Core\\Arr::max`.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One value, of a type with a natural order: a number, a string, a `bool` or \
                   `null`.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other value, of the same type as `$a`.",
            shape: &[],
        },
    ],
    ret: "Whichever compares larger; `$a` on a tie, as PHP's `max` answers too.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When the pair has no natural order — an object, an array, or two values of \
               different kinds.",
    }],
};

/// `Core\Math::clamp`'s reference card — `rule:core-api/reference-card`.
const CLAMP_DOC: MethodDoc = MethodDoc {
    short: "`$n` brought inside `[$low, $high]`, replacing PHP's `min(max($n, $low), $high)` \
            idiom.",
    params: &[
        ParamDoc {
            name: "n",
            desc: "The value to clamp, of a type with a natural order: a number, a string, a \
                   `bool` or `null`.",
            shape: &[],
        },
        ParamDoc {
            name: "low",
            desc: "The smallest value the answer may be.",
            shape: &[],
        },
        ParamDoc {
            name: "high",
            desc: "The largest value the answer may be, at or above `$low`.",
            shape: &[],
        },
    ],
    ret: "`$low` when `$n` is below it, `$high` when `$n` is above it, and `$n` itself \
          otherwise.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When `$low` is above `$high`, which is an empty range, or when any pair has no \
               natural order — an object, an array, or two values of different kinds.",
    }],
};

/// `Core\Math::ceil`'s reference card — `rule:core-api/reference-card`.
const CEIL_DOC: MethodDoc = MethodDoc {
    short: "The smallest integral value at or above `$n`, as `ceil` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to round up.",
        shape: &[],
    }],
    ret: "An integral `float`; `NaN` and the infinities pass through unchanged.",
    errors: &[],
};

/// `Core\Math::floor`'s reference card — `rule:core-api/reference-card`.
const FLOOR_DOC: MethodDoc = MethodDoc {
    short: "The largest integral value at or below `$n`, as `floor` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to round down.",
        shape: &[],
    }],
    ret: "An integral `float`; `NaN` and the infinities pass through unchanged.",
    errors: &[],
};

/// `Core\Math::truncate`'s reference card — `rule:core-api/reference-card`.
const TRUNCATE_DOC: MethodDoc = MethodDoc {
    short: "`$n` with its fractional part dropped — toward zero, so `floor` for a positive `$n` \
            and `ceil` for a negative one — as PHP's `(int)` cast does without the type change.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to truncate.",
        shape: &[],
    }],
    ret: "An integral `float`, still a `float`; `NaN` and the infinities pass through unchanged.",
    errors: &[],
};

/// `Core\Math::round`'s reference card — `rule:core-api/reference-card`.
const ROUND_DOC: MethodDoc = MethodDoc {
    short: "`$n` rounded to `precision` decimal places, with the tie rule named as a \
            `Core\\RoundMode` case, as `round` and its four `PHP_ROUND_*` constants do.",
    params: &[
        ParamDoc {
            name: "n",
            desc: "The value to round.",
            shape: &[],
        },
        ParamDoc {
            name: "precision",
            desc: "Decimal places to keep; zero rounds to an integer and a negative count rounds \
                   to tens, hundreds and up, PHP's `round($n, -2)`.",
            shape: &[],
        },
        ParamDoc {
            name: "mode",
            desc: "Which neighbour a value between two goes to, `RoundMode::HalfUp` — half away \
                   from zero — unless said otherwise.",
            shape: &[],
        },
    ],
    ret: "The nearest `float` to the rounded value; `$n` unchanged when it is `NaN` or an \
          infinity, when `precision` is past `±22`, or when `$n` is already past `2 ** 53` at that \
          precision and has no fraction left to decide.",
    errors: &[],
};

/// `Core\Math::intDiv`'s reference card — `rule:core-api/reference-card`.
const INT_DIV_DOC: MethodDoc = MethodDoc {
    short: "The integer quotient of `$a / $b`, truncated toward zero, as `intdiv` does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The dividend.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The divisor, which may not be zero.",
            shape: &[],
        },
    ],
    ret: "The quotient with any remainder dropped, so `intDiv(-7, 2)` is `-3`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When `$b` is zero, or when `$a` is `INT_MIN` and `$b` is `-1`, whose exact \
               answer is one past `INT_MAX`.",
    }],
};

/// `Core\Math::mod`'s reference card — `rule:core-api/reference-card`.
const MOD_DOC: MethodDoc = MethodDoc {
    short: "The remainder of `$a / $b` over floats, with the sign of `$a`, as `fmod` does; \
            integer modulo is the `%` operator, so this member is the `float` case only.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The dividend.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The divisor, which may not be zero.",
            shape: &[],
        },
    ],
    ret: "`$a - $b * truncate($a / $b)`, carrying `$a`'s sign; `NaN` when `$a` is an infinity, and \
          `$a` unchanged when `$b` is one.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When `$b` is zero — a division by zero, which throws here rather than answering \
               `NaN` as `fmod` does.",
    }],
};

/// `Core\Math::fdiv`'s reference card — `rule:core-api/reference-card`.
const FDIV_DOC: MethodDoc = MethodDoc {
    short: "The IEEE quotient of `$a / $b`, as `fdiv` does — the one member here that answers a \
            zero divisor instead of throwing, the `/` operator having no such spelling.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The dividend.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The divisor, which may be zero.",
            shape: &[],
        },
    ],
    ret: "`$a / $b` under IEEE 754: an infinity signed by both operands when `$b` is zero and \
          `$a` is not, and `NaN` when both are.",
    errors: &[],
};

/// `Core\Math::gcd`'s reference card — `rule:core-api/reference-card`.
const GCD_DOC: MethodDoc = MethodDoc {
    short: "The greatest common divisor of two integers, never negative, as `gmp_gcd` does \
            without the GMP objects.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One integer; its sign is ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other integer; its sign is ignored.",
            shape: &[],
        },
    ],
    ret: "The largest integer dividing both, at least `0`; `gcd(0, 0)` is `0` and \
          `gcd($a, 0)` is `abs($a)`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When the answer does not fit an `int`, which only `gcd(INT_MIN, 0)` and \
               `gcd(INT_MIN, INT_MIN)` reach.",
    }],
};

/// `Core\Math::lcm`'s reference card — `rule:core-api/reference-card`.
const LCM_DOC: MethodDoc = MethodDoc {
    short: "The least common multiple of two integers, never negative, as `gmp_lcm` does \
            without the GMP objects.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One integer; its sign is ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other integer; its sign is ignored.",
            shape: &[],
        },
    ],
    ret: "The smallest positive integer both divide, or `0` when either argument is `0`.",
    errors: &[ErrorDoc {
        error: "ArithmeticError",
        desc: "When the answer is past `INT_MAX`, the usual case for two large coprime \
               arguments.",
    }],
};

/// `Core\Math::sqrt`'s reference card — `rule:core-api/reference-card`.
const SQRT_DOC: MethodDoc = MethodDoc {
    short: "The square root of `$n`, as `sqrt` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to take the root of.",
        shape: &[],
    }],
    ret: "The non-negative root; `NaN` for a negative `$n`, and `-0.0` for `-0.0`.",
    errors: &[],
};

/// `Core\Math::cbrt`'s reference card — `rule:core-api/reference-card`.
const CBRT_DOC: MethodDoc = MethodDoc {
    short: "The cube root of `$n`, replacing PHP's `pow($n, 1/3)` — and defined for a negative \
            `$n`, where that idiom answers `NaN`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to take the root of.",
        shape: &[],
    }],
    ret: "The real cube root, carrying `$n`'s sign; `cbrt(-8.0)` is `-2.0`.",
    errors: &[],
};

/// `Core\Math::hypot`'s reference card — `rule:core-api/reference-card`.
const HYPOT_DOC: MethodDoc = MethodDoc {
    short: "The length of the hypotenuse of a right triangle with legs `$a` and `$b`, as `hypot` \
            does, without the intermediate overflow `sqrt($a ** 2 + $b ** 2)` has.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "One leg.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The other leg.",
            shape: &[],
        },
    ],
    ret: "`sqrt($a * $a + $b * $b)`, never negative, and an infinity when either leg is one.",
    errors: &[],
};

/// `Core\Math::exp`'s reference card — `rule:core-api/reference-card`.
const EXP_DOC: MethodDoc = MethodDoc {
    short: "`E` raised to the power `$n`, as `exp` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The exponent.",
        shape: &[],
    }],
    ret: "`E ** $n`, always positive; `INFINITY` once `$n` is past about `709.78`, and `0.0` \
          far enough below zero.",
    errors: &[],
};

/// `Core\Math::log`'s reference card — `rule:core-api/reference-card`.
const LOG_DOC: MethodDoc = MethodDoc {
    short: "The logarithm of `$n`, natural unless a `base` is given — one member for PHP's \
            `log`, `log10` and `log2`.",
    params: &[
        ParamDoc {
            name: "n",
            desc: "The value to take the logarithm of.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The base, greater than zero; `E` unless said otherwise, and `10.0` and `2.0` \
                   use the dedicated exact routines rather than a ratio of two logarithms.",
            shape: &[],
        },
    ],
    ret: "The power `base` must be raised to for `$n`; `-INFINITY` for a zero `$n`, `NaN` for a \
          negative `$n` and for a `base` of exactly `1.0`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When `base` is not greater than zero, where PHP's `log` raises a `ValueError`.",
    }],
};

/// `Core\Math::sin`'s reference card — `rule:core-api/reference-card`.
const SIN_DOC: MethodDoc = MethodDoc {
    short: "The sine of an angle in radians, as `sin` does.",
    params: &[ParamDoc {
        name: "radians",
        desc: "The angle, in radians.",
        shape: &[],
    }],
    ret: "A value in `[-1, 1]`; `NaN` for an infinity or `NaN`.",
    errors: &[],
};

/// `Core\Math::cos`'s reference card — `rule:core-api/reference-card`.
const COS_DOC: MethodDoc = MethodDoc {
    short: "The cosine of an angle in radians, as `cos` does.",
    params: &[ParamDoc {
        name: "radians",
        desc: "The angle, in radians.",
        shape: &[],
    }],
    ret: "A value in `[-1, 1]`; `NaN` for an infinity or `NaN`.",
    errors: &[],
};

/// `Core\Math::tan`'s reference card — `rule:core-api/reference-card`.
const TAN_DOC: MethodDoc = MethodDoc {
    short: "The tangent of an angle in radians, as `tan` does.",
    params: &[ParamDoc {
        name: "radians",
        desc: "The angle, in radians.",
        shape: &[],
    }],
    ret: "`sin / cos` of the angle, any `float`; `NaN` for an infinity or `NaN`.",
    errors: &[],
};

/// `Core\Math::asin`'s reference card — `rule:core-api/reference-card`.
const ASIN_DOC: MethodDoc = MethodDoc {
    short: "The arc sine — the angle in radians whose sine is `$n` — as `asin` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "A sine, in `[-1, 1]`.",
        shape: &[],
    }],
    ret: "An angle in `[-PI / 2, PI / 2]`; `NaN` for a `$n` outside `[-1, 1]`.",
    errors: &[],
};

/// `Core\Math::acos`'s reference card — `rule:core-api/reference-card`.
const ACOS_DOC: MethodDoc = MethodDoc {
    short: "The arc cosine — the angle in radians whose cosine is `$n` — as `acos` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "A cosine, in `[-1, 1]`.",
        shape: &[],
    }],
    ret: "An angle in `[0, PI]`; `NaN` for a `$n` outside `[-1, 1]`.",
    errors: &[],
};

/// `Core\Math::atan`'s reference card — `rule:core-api/reference-card`.
const ATAN_DOC: MethodDoc = MethodDoc {
    short: "The arc tangent — the angle in radians whose tangent is `$n` — as `atan` does; for a \
            pair of coordinates, `atan2` keeps the quadrant.",
    params: &[ParamDoc {
        name: "n",
        desc: "A tangent, any `float`.",
        shape: &[],
    }],
    ret: "An angle in `(-PI / 2, PI / 2)`, reaching either end for an infinite `$n`.",
    errors: &[],
};

/// `Core\Math::atan2`'s reference card — `rule:core-api/reference-card`.
const ATAN2_DOC: MethodDoc = MethodDoc {
    short: "The angle of the point `($x, $y)` from the positive x-axis, in radians, as `atan2` \
            does — `$y` first, as in PHP and in C.",
    params: &[
        ParamDoc {
            name: "y",
            desc: "The point's y coordinate.",
            shape: &[],
        },
        ParamDoc {
            name: "x",
            desc: "The point's x coordinate.",
            shape: &[],
        },
    ],
    ret: "An angle in `[-PI, PI]`, in the quadrant the two signs choose; `atan2(0.0, 0.0)` is \
          `0.0` rather than `NaN`.",
    errors: &[],
};

/// `Core\Math::sinh`'s reference card — `rule:core-api/reference-card`.
const SINH_DOC: MethodDoc = MethodDoc {
    short: "The hyperbolic sine of `$n`, as `sinh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The argument, any `float`.",
        shape: &[],
    }],
    ret: "`(exp($n) - exp(-$n)) / 2`, carrying `$n`'s sign; an infinity once `$n` is past about \
          `±710`.",
    errors: &[],
};

/// `Core\Math::cosh`'s reference card — `rule:core-api/reference-card`.
const COSH_DOC: MethodDoc = MethodDoc {
    short: "The hyperbolic cosine of `$n`, as `cosh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The argument, any `float`.",
        shape: &[],
    }],
    ret: "`(exp($n) + exp(-$n)) / 2`, at least `1.0`; `INFINITY` once `$n` is past about `±710`.",
    errors: &[],
};

/// `Core\Math::tanh`'s reference card — `rule:core-api/reference-card`.
const TANH_DOC: MethodDoc = MethodDoc {
    short: "The hyperbolic tangent of `$n`, as `tanh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The argument, any `float`.",
        shape: &[],
    }],
    ret: "`sinh / cosh` of `$n`, in `[-1, 1]` and reaching either end for an infinite `$n`.",
    errors: &[],
};

/// `Core\Math::asinh`'s reference card — `rule:core-api/reference-card`.
const ASINH_DOC: MethodDoc = MethodDoc {
    short: "The inverse hyperbolic sine of `$n`, as `asinh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "A hyperbolic sine, any `float`.",
        shape: &[],
    }],
    ret: "The value whose `sinh` is `$n`, carrying `$n`'s sign.",
    errors: &[],
};

/// `Core\Math::acosh`'s reference card — `rule:core-api/reference-card`.
const ACOSH_DOC: MethodDoc = MethodDoc {
    short: "The inverse hyperbolic cosine of `$n`, as `acosh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "A hyperbolic cosine, at least `1.0`.",
        shape: &[],
    }],
    ret: "The non-negative value whose `cosh` is `$n`; `NaN` for a `$n` below `1.0`.",
    errors: &[],
};

/// `Core\Math::atanh`'s reference card — `rule:core-api/reference-card`.
const ATANH_DOC: MethodDoc = MethodDoc {
    short: "The inverse hyperbolic tangent of `$n`, as `atanh` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "A hyperbolic tangent, in `[-1, 1]`.",
        shape: &[],
    }],
    ret: "The value whose `tanh` is `$n`; `INFINITY` at `1.0`, `-INFINITY` at `-1.0` and `NaN` \
          outside them.",
    errors: &[],
};

/// `Core\Math::toRadians`'s reference card — `rule:core-api/reference-card`.
const TO_RADIANS_DOC: MethodDoc = MethodDoc {
    short: "An angle in degrees as radians, as `deg2rad` does — computed as PHP's own \
            `($degrees / 180) * PI`, so a round trip through `toDegrees` agrees with PHP's.",
    params: &[ParamDoc {
        name: "degrees",
        desc: "The angle, in degrees.",
        shape: &[],
    }],
    ret: "The same angle in radians, so `toRadians(180.0)` is `PI`.",
    errors: &[],
};

/// `Core\Math::toDegrees`'s reference card — `rule:core-api/reference-card`.
const TO_DEGREES_DOC: MethodDoc = MethodDoc {
    short: "An angle in radians as degrees, as `rad2deg` does — computed as PHP's own \
            `($radians / PI) * 180`, so a round trip through `toRadians` agrees with PHP's.",
    params: &[ParamDoc {
        name: "radians",
        desc: "The angle, in radians.",
        shape: &[],
    }],
    ret: "The same angle in degrees, so `toDegrees(PI)` is `180.0`.",
    errors: &[],
};

/// `Core\Math::isNan`'s reference card — `rule:core-api/reference-card`.
const IS_NAN_DOC: MethodDoc = MethodDoc {
    short: "Whether `$n` is `NaN` — the one `float` that is not equal to itself, so `==` cannot \
            ask — as `is_nan` does.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to test.",
        shape: &[],
    }],
    ret: "`true` for `NaN` alone; `false` for every other `float`, the infinities included.",
    errors: &[],
};

/// `Core\Math::isFinite`'s reference card — `rule:core-api/reference-card`.
const IS_FINITE_DOC: MethodDoc = MethodDoc {
    short: "Whether `$n` is neither an infinity nor `NaN`, as `is_finite` does; negated and \
            joined with `isNan`, it is `is_infinite` too.",
    params: &[ParamDoc {
        name: "n",
        desc: "The value to test.",
        shape: &[],
    }],
    ret: "`true` for every ordinary `float`, zero included; `false` for `INFINITY`, `-INFINITY` \
          and `NaN`.",
    errors: &[],
};

/// `Core\Math::toBase`'s reference card — `rule:core-api/reference-card`.
const TO_BASE_DOC: MethodDoc = MethodDoc {
    short: "`$n` written out in `$base`, with lowercase digits above nine, as `decbin`, `dechex`, \
            `decoct` and the writing half of `base_convert` do.",
    params: &[
        ParamDoc {
            name: "n",
            desc: "The integer to write.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The radix, from `2` to `36` — the digits and the Latin letters.",
            shape: &[],
        },
    ],
    ret: "The digit string, with no prefix and no padding; a negative `$n` gets a leading `-`, \
          which `base_convert` has no answer for, and `0` is `\"0\"`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "When `$base` is outside `2` to `36`, where `base_convert` answers `0`.",
    }],
};

/// `Core\Math::fromBase`'s reference card — `rule:core-api/reference-card`.
const FROM_BASE_DOC: MethodDoc = MethodDoc {
    short: "The integer `$s` spells in `$base`, case-insensitive above nine, as `bindec`, \
            `hexdec`, `octdec` and the reading half of `base_convert` do — but every digit must \
            belong to the base, where all four of PHP's silently skip one that does not.",
    params: &[
        ParamDoc {
            name: "s",
            desc: "The digit string, with an optional leading `-` and no prefix, whitespace or \
                   grouping.",
            shape: &[],
        },
        ParamDoc {
            name: "base",
            desc: "The radix, from `2` to `36` — the digits and the Latin letters.",
            shape: &[],
        },
    ],
    ret: "The `int` written, the exact inverse of `toBase`, leading `-` included.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "When `$base` is outside `2` to `36`, when `$s` has no digits, or when a \
                   character of `$s` is not a digit of `$base`.",
        },
        ErrorDoc {
            error: "ArithmeticError",
            desc: "When the value does not fit an `int`.",
        },
    ],
};

/// `Core\Math::format`'s reference card — `rule:core-api/reference-card`.
const FORMAT_DOC: MethodDoc = MethodDoc {
    short: "`$n` written for a reader, with a fixed count of decimals and separators the caller \
            names, as `number_format` does — except that grouping is off unless asked for, since \
            Novis has no ambient locale.",
    params: &[
        ParamDoc {
            name: "n",
            desc: "The number to write: an `int`, a finite `float` or a `decimal`.",
            shape: &[],
        },
        ParamDoc {
            name: "decimals",
            desc: "How many fractional digits to write, at most `100`; `0` unless said \
                   otherwise, which writes no decimal separator at all.",
            shape: &[],
        },
        ParamDoc {
            name: "decimalSeparator",
            desc: "The text between the integer and fractional digits, `.` unless said \
                   otherwise, copied into the result verbatim.",
            shape: &[],
        },
        ParamDoc {
            name: "groupSeparator",
            desc: "The text between each group of three integer digits, counted from the right; \
                   empty unless said otherwise, so no grouping happens by default.",
            shape: &[],
        },
    ],
    ret: "The digit string, rounded half away from zero at `decimals` places as `number_format` \
          rounds — exactly for a `decimal` — with a leading `-` for a negative `$n`.",
    errors: &[
        ErrorDoc {
            error: "RuntimeError",
            desc: "When `$n` is an infinity or `NaN`, which have no digits, or when `decimals` \
                   is past `100`.",
        },
        ErrorDoc {
            error: "ArithmeticError",
            desc: "When `$n` is a `uint` past `INT_MAX`.",
        },
    ],
};

/// `Core\Math`'s eleven constants — spec § 3's own list, replacing `M_PI`,
/// `M_E`, `PHP_INT_MAX`, `PHP_FLOAT_EPSILON` and the rest of PHP's global
/// constants under `rule:classes/no-free-functions-or-constants`.
///
/// **Every one is written as the value, not as an expression.** `TAU` is
/// spelled out rather than `2.0 * PI` and `EPSILON` rather than an
/// `f64::EPSILON` reference, because a `Const` is a literal the compiler
/// inlines at the use site and there is nowhere for a computation to happen;
/// the two that *are* named through Rust — `i64::MAX` and `u64::MAX` — are
/// exact integers, where a written digit string could not be checked at all.
///
/// `FLOAT_MIN` is the smallest **positive normal** `f64`, which is PHP's
/// `PHP_FLOAT_MIN` and not `f64::MIN`: the negative extreme is
/// `0.0 - FLOAT_MAX`, and giving that name to the tiny value is the one
/// place this roster follows PHP rather than Rust.
const CONSTANTS: &[CoreConst] = &[
    CoreConst {
        name: "PI",
        ty: CoreTy::Float,
        value: Const::Float(std::f64::consts::PI),
        desc: "The ratio of a circle's circumference to its diameter, `3.14159…` as the nearest \
               `float` — PHP's `M_PI`.",
    },
    CoreConst {
        name: "TAU",
        ty: CoreTy::Float,
        value: Const::Float(std::f64::consts::TAU),
        desc: "The ratio of a circle's circumference to its radius, `2 * PI` or `6.28318…` as the \
               nearest `float` — a full turn in radians, which PHP spells `2 * M_PI`.",
    },
    CoreConst {
        name: "E",
        ty: CoreTy::Float,
        value: Const::Float(std::f64::consts::E),
        desc: "Euler's number, the base of the natural logarithm, `2.71828…` as the nearest \
               `float` — PHP's `M_E`, and `log`'s default base.",
    },
    CoreConst {
        name: "EPSILON",
        ty: CoreTy::Float,
        value: Const::Float(f64::EPSILON),
        desc: "The smallest `float` that added to `1.0` gives a value other than `1.0`, \
               `2.220446049250313e-16` — PHP's `PHP_FLOAT_EPSILON`, and the tolerance to compare \
               two floats with in place of `==`.",
    },
    CoreConst {
        name: "INT_MAX",
        ty: CoreTy::Int,
        value: Const::Int(i64::MAX),
        desc: "The largest `int`, `9223372036854775807` — PHP's `PHP_INT_MAX`.",
    },
    CoreConst {
        name: "INT_MIN",
        ty: CoreTy::Int,
        value: Const::Int(i64::MIN),
        desc: "The smallest `int`, `-9223372036854775808` — PHP's `PHP_INT_MIN`, and the one \
               value `abs` has no `int` answer for.",
    },
    CoreConst {
        name: "UINT_MAX",
        ty: CoreTy::Uint,
        value: Const::Uint(u64::MAX),
        desc: "The largest `uint`, `18446744073709551615` — a type PHP does not have, so there \
               is no constant to replace.",
    },
    CoreConst {
        name: "FLOAT_MAX",
        ty: CoreTy::Float,
        value: Const::Float(f64::MAX),
        desc: "The largest finite `float`, `1.7976931348623157e308` — PHP's `PHP_FLOAT_MAX`; \
               the negative extreme is `-FLOAT_MAX`.",
    },
    CoreConst {
        name: "FLOAT_MIN",
        ty: CoreTy::Float,
        value: Const::Float(f64::MIN_POSITIVE),
        desc: "The smallest positive normal `float`, `2.2250738585072014e-308` — PHP's \
               `PHP_FLOAT_MIN`, and not the negative extreme, which is `-FLOAT_MAX`.",
    },
    CoreConst {
        name: "NAN",
        ty: CoreTy::Float,
        value: Const::Float(f64::NAN),
        desc: "The `float` that is not a number — PHP's `NAN` — equal to nothing, itself \
               included, so `isNan` is the only test for it.",
    },
    CoreConst {
        name: "INFINITY",
        ty: CoreTy::Float,
        value: Const::Float(f64::INFINITY),
        desc: "Positive infinity as a `float`, past every finite value — PHP's `INF`; the \
               negative one is `-INFINITY`, and `isFinite` is `false` for both.",
    },
];

/// `Core\RoundMode` — spec § 3's six rounding rules, replacing PHP's four
/// `PHP_ROUND_HALF_*` global constants plus the two whole-direction rules
/// `ceil`/`floor` only cover for a precision of zero.
///
/// Flat under `Core`, following `Core\Order`: an enum a `Core` member takes is
/// part of `Core`'s surface, and nesting it under the class that takes it
/// would be the only nested name in the roster.
pub const ROUND_MODE: CoreEnum = CoreEnum {
    name: r"Core\RoundMode",
    cases: &[
        ("HalfUp", 0),
        ("HalfDown", 1),
        ("HalfEven", 2),
        ("HalfOdd", 3),
        ("Up", 4),
        ("Down", 5),
    ],
    doc: Some(&ROUND_MODE_DOC),
};

/// [`ROUND_MODE`]'s reference card — `rule:core-api/reference-card`, one line per case, and
/// the semantics are [`RoundMode`]'s, which `round_with` implements.
const ROUND_MODE_DOC: EnumDoc = EnumDoc {
    short: "How `Core\\Math::round` settles a value between two neighbours — PHP's four \
            `PHP_ROUND_HALF_*` constants plus the two whole-direction rules `ceil` and `floor` \
            only cover at a precision of zero.",
    cases: &[
        CaseDoc {
            name: "HalfUp",
            desc: "A tie goes away from zero — `PHP_ROUND_HALF_UP`, and `round`'s default \
                   there and here.",
        },
        CaseDoc {
            name: "HalfDown",
            desc: "A tie goes toward zero — `PHP_ROUND_HALF_DOWN`.",
        },
        CaseDoc {
            name: "HalfEven",
            desc: "A tie goes to the even neighbour — banker's rounding, `PHP_ROUND_HALF_EVEN`, \
                   the rule that does not accumulate a bias over many values.",
        },
        CaseDoc {
            name: "HalfOdd",
            desc: "A tie goes to the odd neighbour — `PHP_ROUND_HALF_ODD`.",
        },
        CaseDoc {
            name: "Up",
            desc: "Every value goes away from zero, tie or not — `ceil` for a positive number \
                   and `floor` for a negative one, at any precision.",
        },
        CaseDoc {
            name: "Down",
            desc: "Every value goes toward zero, tie or not — truncation, at any precision.",
        },
    ],
};

/// `int|float|decimal` — spec § 3's own union, at the three members here that
/// take it whole and at `Core\Arr`'s three aggregations, which write the same
/// row. See this module's own gap note for the four members that do not take
/// it yet.
pub(crate) const NUMBER: &[CoreTy] = &[CoreTy::Int, CoreTy::Float, CoreTy::Decimal];

/// `Core\Math::round`'s `{precision?: int, mode?: RoundMode}`.
///
/// `precision` is `int` rather than `uint` on purpose: a negative one rounds
/// to tens, hundreds and up, which is `round($n, -2)` in PHP and the only
/// spelling for it here.
const ROUND_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "precision",
        ty: CoreTy::Int,
        default: Const::Int(0),
    },
    CoreOption {
        name: "mode",
        ty: CoreTy::Enum(r"Core\RoundMode"),
        default: Const::EnumCase(r"Core\RoundMode", "HalfUp"),
    },
];

/// `Core\Math::log`'s `{base?: float}` — the option that folds PHP's `log10`
/// and `log2` into one member, with the natural logarithm as the default.
const LOG_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "base",
    ty: CoreTy::Float,
    default: Const::Float(std::f64::consts::E),
}];

/// `Core\Math::format`'s
/// `{decimals?: uint, decimalSeparator?: string, groupSeparator?: string}`.
///
/// The group separator defaults to **empty**, unlike `number_format`'s `,`:
/// Novis has no ambient locale (spec § 3), so grouping is a thing the caller
/// asks for rather than a thing it has to switch off.
///
/// **Both separators are [`Qual::Contagious`]**, and the spec's Q column said
/// *neutral* until this row was classified. It is the one cell in this class
/// that a member-wide mark could not render: `fromBase` and the rest of
/// `Core\Math` answer numbers, so nothing an argument holds reaches the answer
/// and *neutral* is right for the class — but `format` answers a `string` that
/// contains these two options **verbatim**, which is the first bullet of
/// [`Qual`]'s rule failing on its own terms. The registry is the home of the
/// classification (`rule:security/unclassified-parameter-refuses-tainted`) and the spec renders it, so the cell was
/// corrected rather than the mark.
const FORMAT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "decimals",
        ty: CoreTy::Uint,
        default: Const::Uint(0),
    },
    CoreOption {
        name: "decimalSeparator",
        ty: CoreTy::Text(Qual::Contagious),
        default: Const::Str("."),
    },
    CoreOption {
        name: "groupSeparator",
        ty: CoreTy::Text(Qual::Contagious),
        default: Const::Str(""),
    },
];

/// This module's own symbols, for [`crate::symbols`] to ask about — one arm
/// per member, and no other module answers for any of them.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_math_abs" => (nvs_core_math_abs as *const ()).cast(),
        "nvs_core_math_sign" => (nvs_core_math_sign as *const ()).cast(),
        "nvs_core_math_min" => (nvs_core_math_min as *const ()).cast(),
        "nvs_core_math_max" => (nvs_core_math_max as *const ()).cast(),
        "nvs_core_math_clamp" => (nvs_core_math_clamp as *const ()).cast(),
        "nvs_core_math_ceil" => (nvs_core_math_ceil as *const ()).cast(),
        "nvs_core_math_floor" => (nvs_core_math_floor as *const ()).cast(),
        "nvs_core_math_truncate" => (nvs_core_math_truncate as *const ()).cast(),
        "nvs_core_math_round" => (nvs_core_math_round as *const ()).cast(),
        "nvs_core_math_int_div" => (nvs_core_math_int_div as *const ()).cast(),
        "nvs_core_math_mod" => (nvs_core_math_mod as *const ()).cast(),
        "nvs_core_math_fdiv" => (nvs_core_math_fdiv as *const ()).cast(),
        "nvs_core_math_gcd" => (nvs_core_math_gcd as *const ()).cast(),
        "nvs_core_math_lcm" => (nvs_core_math_lcm as *const ()).cast(),
        "nvs_core_math_sqrt" => (nvs_core_math_sqrt as *const ()).cast(),
        "nvs_core_math_cbrt" => (nvs_core_math_cbrt as *const ()).cast(),
        "nvs_core_math_hypot" => (nvs_core_math_hypot as *const ()).cast(),
        "nvs_core_math_exp" => (nvs_core_math_exp as *const ()).cast(),
        "nvs_core_math_log" => (nvs_core_math_log as *const ()).cast(),
        "nvs_core_math_sin" => (nvs_core_math_sin as *const ()).cast(),
        "nvs_core_math_cos" => (nvs_core_math_cos as *const ()).cast(),
        "nvs_core_math_tan" => (nvs_core_math_tan as *const ()).cast(),
        "nvs_core_math_asin" => (nvs_core_math_asin as *const ()).cast(),
        "nvs_core_math_acos" => (nvs_core_math_acos as *const ()).cast(),
        "nvs_core_math_atan" => (nvs_core_math_atan as *const ()).cast(),
        "nvs_core_math_atan2" => (nvs_core_math_atan2 as *const ()).cast(),
        "nvs_core_math_sinh" => (nvs_core_math_sinh as *const ()).cast(),
        "nvs_core_math_cosh" => (nvs_core_math_cosh as *const ()).cast(),
        "nvs_core_math_tanh" => (nvs_core_math_tanh as *const ()).cast(),
        "nvs_core_math_asinh" => (nvs_core_math_asinh as *const ()).cast(),
        "nvs_core_math_acosh" => (nvs_core_math_acosh as *const ()).cast(),
        "nvs_core_math_atanh" => (nvs_core_math_atanh as *const ()).cast(),
        "nvs_core_math_to_radians" => (nvs_core_math_to_radians as *const ()).cast(),
        "nvs_core_math_to_degrees" => (nvs_core_math_to_degrees as *const ()).cast(),
        "nvs_core_math_is_nan" => (nvs_core_math_is_nan as *const ()).cast(),
        "nvs_core_math_is_finite" => (nvs_core_math_is_finite as *const ()).cast(),
        "nvs_core_math_to_base" => (nvs_core_math_to_base as *const ()).cast(),
        "nvs_core_math_from_base" => (nvs_core_math_from_base as *const ()).cast(),
        "nvs_core_math_format" => (nvs_core_math_format as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Argument decoding — a wrong tag here is a compiler bug, so it is `fatal`
// ============================================================================

/// The `float` at `index`, or the `FATAL` a wrong tag is.
///
/// Every one of these decoders reports `Fault::fatal` rather than
/// `Fault::thrown`, on `crate::arr`'s convention: the checker has already
/// proved the argument's type, so a mismatched tag is a compiler defect and
/// not something an Novis program can provoke or catch.
fn float_at(args: &[Value], index: usize, member: &str) -> Result<f64, Fault> {
    args[index].as_float().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Math::{member} expected {:?} at argument {index}, got tag {}",
            Tag::Float,
            args[index].tag_byte()
        ))
    })
}

/// The `int` at `index`, or the `FATAL` a wrong tag is.
fn int_at(args: &[Value], index: usize, member: &str) -> Result<i64, Fault> {
    args[index].as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Math::{member} expected {:?} at argument {index}, got tag {}",
            Tag::Int,
            args[index].tag_byte()
        ))
    })
}

/// The `uint` at `index`, or the `FATAL` a wrong tag is.
fn uint_at(args: &[Value], index: usize, member: &str) -> Result<u64, Fault> {
    args[index].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Math::{member} expected {:?} at argument {index}, got tag {}",
            Tag::Uint,
            args[index].tag_byte()
        ))
    })
}

/// The `string` bytes at `index`, or the `FATAL` a wrong tag is.
fn str_at<'a>(args: &'a [Value], index: usize, member: &str) -> Result<&'a [u8], Fault> {
    args[index].as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Math::{member} expected {:?} at argument {index}, got tag {}",
            Tag::Str,
            args[index].tag_byte()
        ))
    })
}

/// One argument of the spec's `int|float` union, decoded.
///
/// A `uint` is accepted as well as an `int` even though the declared union
/// holds neither `rule:types/arithmetic` type twice: a `uint` reaches a union parameter
/// through the same tagged slot, and refusing it here would be a `FATAL` for
/// a value the member has an exact answer for.
#[derive(Clone, Copy)]
enum Number {
    /// An `int` or a `uint` small enough to be one.
    Integer(i64),
    /// A `float`.
    Real(f64),
    /// A `decimal` — `rule:types/decimal`'s scalar, and the one arm that is exact.
    Exact(Decimal),
}

fn number_at(args: &[Value], index: usize, member: &str) -> Result<Number, Fault> {
    if let Some(int) = args[index].as_int() {
        return Ok(Number::Integer(int));
    }
    if let Some(uint) = args[index].as_uint() {
        return i64::try_from(uint).map(Number::Integer).map_err(|_| {
            Fault::thrown_as(
                ThrownClass::Arithmetic,
                format!(
                    "Core\\Math::{member} cannot take {uint}: it is past `int`'s largest value"
                ),
            )
        });
    }
    if let Some(float) = args[index].as_float() {
        return Ok(Number::Real(float));
    }
    if let Some(exact) = args[index].as_decimal() {
        return Ok(Number::Exact(exact));
    }
    Err(Fault::fatal(format!(
        "Core\\Math::{member} expected a number at argument {index}, got tag {}",
        args[index].tag_byte()
    )))
}

// ============================================================================
// The members
// ============================================================================

/// Defines a one-`float`-in, one-`float`-out member — the shape twenty of
/// spec § 3's rows share exactly.
///
/// Written as a macro rather than twenty copies of the same four lines
/// because the *only* thing that differs between them is the `f64` method,
/// and a copy per member is a copy per member to get wrong. The module's own
/// docs own the rule they all obey: a domain error is IEEE's answer.
macro_rules! unary_float {
    ($(#[$meta:meta])* $name:ident, $member:literal, $op:expr) => {
        nvs_runtime::nvs_helper! {
            $(#[$meta])*
            fn $name(_ctx, args: [1]) {
                let operation: fn(f64) -> f64 = $op;
                Ok(Value::float(operation(float_at(args, 0, $member)?)))
            }
        }
    };
}

unary_float! {
    /// `Core\Math::ceil(float $n): float` — the smallest integral value at or
    /// above `$n`, replacing PHP's `ceil`.
    nvs_core_math_ceil, "ceil", f64::ceil
}

unary_float! {
    /// `Core\Math::floor(float $n): float` — the largest integral value at or
    /// below `$n`, replacing PHP's `floor`.
    nvs_core_math_floor, "floor", f64::floor
}

unary_float! {
    /// `Core\Math::truncate(float $n): float` — `$n` with its fractional part
    /// dropped, replacing PHP's `(int)` truncation. Toward zero, so it is
    /// `floor` for a positive `$n` and `ceil` for a negative one.
    nvs_core_math_truncate, "truncate", f64::trunc
}

unary_float! {
    /// `Core\Math::sqrt(float $n): float` — the square root, replacing PHP's
    /// `sqrt`. `NaN` below zero.
    nvs_core_math_sqrt, "sqrt", f64::sqrt
}

unary_float! {
    /// `Core\Math::cbrt(float $n): float` — the cube root, replacing PHP's
    /// `pow($n, 1/3)`, which is `NaN` for a negative `$n` where this is not.
    nvs_core_math_cbrt, "cbrt", f64::cbrt
}

unary_float! {
    /// `Core\Math::exp(float $n): float` — `e` to the power of `$n`, replacing
    /// PHP's `exp`.
    nvs_core_math_exp, "exp", f64::exp
}

unary_float! {
    /// `Core\Math::sin(float $radians): float` — replacing PHP's `sin`.
    nvs_core_math_sin, "sin", f64::sin
}

unary_float! {
    /// `Core\Math::cos(float $radians): float` — replacing PHP's `cos`.
    nvs_core_math_cos, "cos", f64::cos
}

unary_float! {
    /// `Core\Math::tan(float $radians): float` — replacing PHP's `tan`.
    nvs_core_math_tan, "tan", f64::tan
}

unary_float! {
    /// `Core\Math::asin(float $n): float` — replacing PHP's `asin`. `NaN`
    /// outside `[-1, 1]`.
    nvs_core_math_asin, "asin", f64::asin
}

unary_float! {
    /// `Core\Math::acos(float $n): float` — replacing PHP's `acos`. `NaN`
    /// outside `[-1, 1]`.
    nvs_core_math_acos, "acos", f64::acos
}

unary_float! {
    /// `Core\Math::atan(float $n): float` — replacing PHP's `atan`.
    nvs_core_math_atan, "atan", f64::atan
}

unary_float! {
    /// `Core\Math::sinh(float $n): float` — replacing PHP's `sinh`.
    nvs_core_math_sinh, "sinh", f64::sinh
}

unary_float! {
    /// `Core\Math::cosh(float $n): float` — replacing PHP's `cosh`.
    nvs_core_math_cosh, "cosh", f64::cosh
}

unary_float! {
    /// `Core\Math::tanh(float $n): float` — replacing PHP's `tanh`.
    nvs_core_math_tanh, "tanh", f64::tanh
}

unary_float! {
    /// `Core\Math::asinh(float $n): float` — replacing PHP's `asinh`.
    nvs_core_math_asinh, "asinh", f64::asinh
}

unary_float! {
    /// `Core\Math::acosh(float $n): float` — replacing PHP's `acosh`. `NaN`
    /// below `1`.
    nvs_core_math_acosh, "acosh", f64::acosh
}

unary_float! {
    /// `Core\Math::atanh(float $n): float` — replacing PHP's `atanh`. An
    /// infinity at `±1` and `NaN` outside them.
    nvs_core_math_atanh, "atanh", f64::atanh
}

unary_float! {
    /// `Core\Math::toRadians(float $degrees): float` — replacing PHP's
    /// `deg2rad`.
    ///
    /// Written as `($degrees / 180.0) * PI` rather than as `f64::to_radians`,
    /// which multiplies by the correctly rounded constant `PI / 180.0`. The
    /// two part by at most one ulp, and the std form is the more accurate of
    /// them — over the 3,600 tenths of a degree in a turn it is closer to the
    /// true value 851 times against 118 — but this is PHP's own expression,
    /// and matching it is what makes the round trip below agree. AGENTS.md's
    /// priority 2 is PHP-compatible *observable* behaviour, and the ulp is
    /// observable: `==` over `float` is exact ([ADR
    /// 0090](/docs/decisions/0090.md)),
    /// so `toDegrees(toRadians(30.0)) == 30.0` answers `true` under PHP's
    /// spelling and `false` under the std one. Nothing here promises an
    /// accuracy the twin does not have; the round trip a ported program
    /// already wrote is the thing worth keeping.
    nvs_core_math_to_radians, "toRadians", |degrees| (degrees / 180.0) * std::f64::consts::PI
}

unary_float! {
    /// `Core\Math::toDegrees(float $radians): float` — replacing PHP's
    /// `rad2deg`.
    ///
    /// `($radians / PI) * 180.0`, PHP's expression, for the reason
    /// `toRadians` above states in full. This is the half of the pair the
    /// difference is visible through: `f64::to_degrees` and this form part
    /// over a whole-degree angle's radians, which is exactly what a round
    /// trip feeds it.
    nvs_core_math_to_degrees, "toDegrees", |radians| (radians / std::f64::consts::PI) * 180.0
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::abs(int|float $n): int|float` — the magnitude, replacing
    /// PHP's `abs`, and the one member here whose result type is the argument's
    /// own.
    ///
    /// `abs(int::MIN)` throws: its magnitude is one past `int`'s largest
    /// value, and PHP's answer — silently becoming a `float` — is exactly the
    /// by-itself type change `rule:types/declaration` forbids.
    fn nvs_core_math_abs(_ctx, args: [1]) {
        Ok(match number_at(args, 0, "abs")? {
            Number::Integer(n) => Value::int(n.checked_abs().ok_or_else(|| {
                Fault::thrown_as(
                    ThrownClass::Arithmetic,
                    "Core\\Math::abs has no `int` answer for `int`'s smallest value: its \
                     magnitude is one past the largest",
                )
            })?),
            Number::Real(n) => Value::float(n.abs()),
            // Total, unlike the `int` arm: the mantissa is unsigned, so there
            // is no magnitude that does not fit.
            Number::Exact(n) => Value::decimal(n.abs()),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::sign(int|float $n): int` — `-1`, `0` or `1`, replacing
    /// PHP's `$n <=> 0`.
    ///
    /// `-0.0` is `0`, since it *is* zero. `NaN` throws rather than answering:
    /// it is on neither side of zero, and every other answer this member could
    /// give would be a value the caller would go on to branch on.
    fn nvs_core_math_sign(_ctx, args: [1]) {
        let sign = match number_at(args, 0, "sign")? {
            Number::Integer(n) => n.signum(),
            Number::Real(n) if n.is_nan() => {
                return Err(Fault::thrown(
                    "Core\\Math::sign has no answer for `NaN`: it is on neither side of zero"
                        .to_owned(),
                ));
            }
            // Zero is never negative for a `decimal` (`nvs_runtime::decimal`),
            // so this needs no `-0.0` case the way the `float` arm below does.
            Number::Exact(n) if n.is_zero() => 0,
            Number::Exact(n) if n.is_negative() => -1,
            Number::Exact(_) => 1,
            Number::Real(0.0) => 0,
            Number::Real(n) => {
                if n.is_sign_negative() {
                    -1
                } else {
                    1
                }
            }
        };
        Ok(Value::int(sign))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::min(T $a, T $b): T` — the smaller of two values, replacing
    /// PHP's `min` with scalar arguments.
    ///
    /// The ordering is `crate::ordering::compare_values`, the same one
    /// `Core\Arr::min` walks an array with, so `Math::min($a, $b)` and
    /// `Arr::min([$a, $b])` cannot answer differently. A tie answers `$a`.
    ///
    /// PHP's two-argument `min` answers `$b` on a tie and its `max` answers
    /// `$a`, so `min(1000000000000000000, 1.0e18)` hands back the `float`
    /// there and the `int` here. Both members answering the same end is worth
    /// more than matching that, since the alternative is `min` and `max`
    /// disagreeing with each other about which of two equal values is meant;
    /// `math-min-and-max-diverge-from-php-over-a-tie-a-numeral-string-or-an-unordered-pair`
    /// pins it.
    ///
    /// PHP's variadic `min(1, 2, 3)` has no member: the two-argument form
    /// nests, and the array form is `Core\Arr::min`.
    fn nvs_core_math_min(ctx, args: [2]) {
        pick(ctx, args, std::cmp::Ordering::Less, "Core\\Math::min")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::max(T $a, T $b): T` — the larger of two values, replacing
    /// PHP's `max` with scalar arguments. [`nvs_core_math_min`] owns the
    /// ordering and the tie rule.
    fn nvs_core_math_max(ctx, args: [2]) {
        pick(ctx, args, std::cmp::Ordering::Greater, "Core\\Math::max")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::clamp(T $n, T $low, T $high): T` — `$n` brought inside
    /// `[$low, $high]`, replacing the `min(max(…))` idiom.
    ///
    /// A `$low` above `$high` throws rather than silently answering one of
    /// them: the interval is empty, so there is no value to clamp *into*, and
    /// both of PHP's idiomatic spellings answer a different one.
    fn nvs_core_math_clamp(ctx, args: [3]) {
        const MEMBER: &str = "Core\\Math::clamp";
        if compare_values(ctx, &args[1], &args[2], MEMBER)? == std::cmp::Ordering::Greater {
            return Err(Fault::thrown(
                "Core\\Math::clamp was given a `low` above its `high`, which is an empty range"
                    .to_owned(),
            ));
        }
        let chosen = if compare_values(ctx, &args[0], &args[1], MEMBER)? == std::cmp::Ordering::Less
        {
            args[1]
        } else if compare_values(ctx, &args[0], &args[2], MEMBER)? == std::cmp::Ordering::Greater {
            args[2]
        } else {
            args[0]
        };
        Ok(owned(chosen))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::round(float $n, {precision?: int, mode?: RoundMode}): float`
    /// — replacing PHP's `round` and its four `PHP_ROUND_*` constants.
    ///
    /// [`round_to`] owns what each mode means and what a precision does.
    fn nvs_core_math_round(_ctx, args: [3]) {
        let value = float_at(args, 0, "round")?;
        let precision = int_at(args, 1, "round")?;
        let mode = round_mode(args, 2, "Core\\Math::round")?;
        Ok(Value::float(round_to(value, precision, mode)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::intDiv(int $a, int $b): int` — the quotient, truncated
    /// toward zero, replacing PHP's `intdiv`.
    ///
    /// A zero divisor throws, per spec § 3, and so does `int::MIN / -1`, whose
    /// exact answer is one past `int`'s largest value.
    fn nvs_core_math_int_div(_ctx, args: [2]) {
        let a = int_at(args, 0, "intDiv")?;
        let b = int_at(args, 1, "intDiv")?;
        a.checked_div(b)
            .map(Value::int)
            .ok_or_else(|| divide_by_zero_or_overflow("intDiv", b))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::mod(float $a, float $b): float` — the remainder of `$a / $b`
    /// with the sign of `$a`, replacing PHP's `fmod`; integer modulo is the
    /// `%` operator, which is why this member is the `float` case only.
    ///
    /// A zero divisor throws rather than answering `NaN` the way `fmod` does:
    /// it is a division by zero, which spec § 3 makes a throw, and not one of
    /// the domain errors this module's docs keep at IEEE's answer.
    fn nvs_core_math_mod(_ctx, args: [2]) {
        let a = float_at(args, 0, "mod")?;
        let b = float_at(args, 1, "mod")?;
        if b == 0.0 {
            return Err(Fault::thrown_as(
                ThrownClass::Arithmetic,
                "Core\\Math::mod was given a zero divisor",
            ));
        }
        Ok(Value::float(a % b))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::fdiv(float $a, float $b): float` — the IEEE quotient,
    /// replacing PHP's `fdiv`, and the one division in this module that
    /// answers a zero divisor rather than throwing on it.
    ///
    /// It exists because `rule:types/arithmetic` refuses the zero divisor for `/` before
    /// the operand types are consulted, so an infinity is otherwise
    /// unreachable through a division — which is the same reason PHP grew this
    /// function once `/` began throwing. That is a member *named* for the
    /// answer rather than this module's domain-error rule reaching a division:
    /// every other division here still throws, and a reader who writes `/`
    /// still gets the throw.
    fn nvs_core_math_fdiv(_ctx, args: [2]) {
        let a = float_at(args, 0, "fdiv")?;
        let b = float_at(args, 1, "fdiv")?;
        Ok(Value::float(a / b))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::gcd(int $a, int $b): int` — the greatest common divisor,
    /// never negative, replacing `gmp_gcd`.
    ///
    /// `gcd(0, 0)` is `0`, the conventional answer: every integer divides
    /// zero, so the set of common divisors is unbounded and zero is the
    /// identity that keeps `gcd(gcd($a, $b), $c)` associative.
    ///
    /// The walk is over `i128` so that `int::MIN`, whose magnitude does not
    /// fit an `i64`, needs no special case; the result always does, because a
    /// divisor of `int::MIN` is at most its magnitude and the only one that
    /// large is `int::MIN` itself paired with zero — which is why the
    /// conversion back throws instead of asserting.
    fn nvs_core_math_gcd(_ctx, args: [2]) {
        let a = i128::from(int_at(args, 0, "gcd")?);
        let b = i128::from(int_at(args, 1, "gcd")?);
        i64::try_from(gcd(a.abs(), b.abs()))
            .map(Value::int)
            .map_err(|_| does_not_fit("gcd"))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::lcm(int $a, int $b): int` — the least common multiple,
    /// never negative, replacing `gmp_lcm`.
    ///
    /// `lcm($a, 0)` is `0`. A result past `int`'s largest value throws, which
    /// is the common case for two large coprime arguments.
    fn nvs_core_math_lcm(_ctx, args: [2]) {
        let a = i128::from(int_at(args, 0, "lcm")?).abs();
        let b = i128::from(int_at(args, 1, "lcm")?).abs();
        if a == 0 || b == 0 {
            return Ok(Value::int(0));
        }
        i64::try_from(a / gcd(a, b) * b)
            .map(Value::int)
            .map_err(|_| does_not_fit("lcm"))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::hypot(float $a, float $b): float` — the hypotenuse without
    /// the intermediate overflow `sqrt($a ** 2 + $b ** 2)` has, replacing
    /// PHP's `hypot`.
    fn nvs_core_math_hypot(_ctx, args: [2]) {
        let a = float_at(args, 0, "hypot")?;
        let b = float_at(args, 1, "hypot")?;
        Ok(Value::float(a.hypot(b)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::atan2(float $y, float $x): float` — the angle of `($x, $y)`
    /// from the positive x-axis, replacing PHP's `atan2`. `$y` first, as in
    /// PHP and in C.
    fn nvs_core_math_atan2(_ctx, args: [2]) {
        let y = float_at(args, 0, "atan2")?;
        let x = float_at(args, 1, "atan2")?;
        Ok(Value::float(y.atan2(x)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::log(float $n, {base?: float}): float` — the logarithm,
    /// replacing PHP's `log`, `log10` and `log2`.
    ///
    /// Base `10` and base `2` take `f64`'s own dedicated routines rather than
    /// `ln($n) / ln($base)`, which is a ratio of two inexact values and
    /// answers `1.9999999999999998` for `log(100.0, {base: 10.0})`. Every
    /// other base is that ratio, because there is nothing more exact to use.
    ///
    /// A base that is not greater than zero has no logarithm at all, and the
    /// ratio would answer one anyway — `ln($n) / ln(0.0)` is a signed zero and
    /// `ln($n) / ln(-2.0)` a `NaN` — so it is refused rather than repaired,
    /// the way spec § 3 refuses a zero divisor. Base `1.0` is `NaN`: the ratio
    /// divides by zero there and would answer an infinity whose sign is the
    /// argument's. Both rules are PHP's, which raises a `ValueError` for the
    /// first and answers `NAN` to the second; only the class of the refusal
    /// differs. The *argument's* domain is left at IEEE's answer, matching
    /// PHP's `log` exactly: zero is `-INF` and a negative is `NaN`.
    fn nvs_core_math_log(_ctx, args: [2]) {
        let n = float_at(args, 0, "log")?;
        let base = float_at(args, 1, "log")?;
        let result = if base == std::f64::consts::E {
            n.ln()
        } else if base == 10.0 {
            n.log10()
        } else if base == 2.0 {
            n.log2()
        } else if base <= 0.0 {
            return Err(Fault::thrown(
                "Core\\Math::log was given a base that is not greater than zero".to_owned(),
            ));
        } else if base == 1.0 {
            f64::NAN
        } else {
            n.log(base)
        };
        Ok(Value::float(result))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::isNan(float $n): bool` — whether `$n` is the one value
    /// that is not equal to itself, replacing PHP's `is_nan`.
    fn nvs_core_math_is_nan(_ctx, args: [1]) {
        Ok(Value::bool(float_at(args, 0, "isNan")?.is_nan()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::isFinite(float $n): bool` — whether `$n` is neither an
    /// infinity nor `NaN`, replacing PHP's `is_finite` and, negated together
    /// with [`nvs_core_math_is_nan`], its `is_infinite`.
    fn nvs_core_math_is_finite(_ctx, args: [1]) {
        Ok(Value::bool(float_at(args, 0, "isFinite")?.is_finite()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::toBase(int $n, uint $base): string` — `$n` written in
    /// `$base`, replacing PHP's `decbin`, `dechex`, `decoct` and the
    /// to-half of `base_convert`.
    ///
    /// Digits above nine are **lowercase**, as `dechex` writes them. A
    /// negative `$n` is written with a leading `-`, which `base_convert` has
    /// no answer for at all; the magnitude is taken through `i128` so
    /// `int::MIN` needs no special case.
    fn nvs_core_math_to_base(_ctx, args: [2]) {
        let n = int_at(args, 0, "toBase")?;
        let base = radix(uint_at(args, 1, "toBase")?, "toBase")?;
        let mut digits = Vec::new();
        let mut magnitude = i128::from(n).abs();
        if magnitude == 0 {
            digits.push(b'0');
        }
        while magnitude > 0 {
            let digit = usize::try_from(magnitude % i128::from(base))
                .expect("a remainder of a base at most 36 is a small index");
            digits.push(DIGITS[digit]);
            magnitude /= i128::from(base);
        }
        if n < 0 {
            digits.push(b'-');
        }
        digits.reverse();
        Ok(Value::str(NvsStr::new(&digits)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::fromBase(string $s, uint $base): int` — the integer `$s`
    /// spells in `$base`, replacing PHP's `bindec`, `hexdec`, `octdec` and the
    /// from-half of `base_convert`.
    ///
    /// Case-insensitive above nine, and the exact inverse of
    /// [`nvs_core_math_to_base`], leading `-` included. Every one of PHP's
    /// four **silently ignores** a digit the base has no room for — `hexdec`
    /// reads `"beefy"` as `48879` — which is precisely the quiet wrong answer
    /// `rule:core-api/shape-rules` R4 makes a throw here, along with an empty string and a result
    /// past `int`.
    fn nvs_core_math_from_base(_ctx, args: [2]) {
        let text = str_at(args, 0, "fromBase")?;
        let base = radix(uint_at(args, 1, "fromBase")?, "fromBase")?;
        let (negative, digits) = match text.split_first() {
            Some((b'-', rest)) => (true, rest),
            _ => (false, text),
        };
        if digits.is_empty() {
            return Err(Fault::thrown(
                "Core\\Math::fromBase was given no digits to read".to_owned(),
            ));
        }
        let mut magnitude: i128 = 0;
        for byte in digits {
            let digit = DIGITS
                .iter()
                .position(|candidate| *candidate == byte.to_ascii_lowercase())
                .filter(|digit| u64::try_from(*digit).expect("a digit index is small") < base)
                .ok_or_else(|| {
                    Fault::thrown(format!(
                        "Core\\Math::fromBase found {} in a base-{base} number, which has no \
                         digit for it",
                        readable(*byte)
                    ))
                })?;
            magnitude = magnitude * i128::from(base) + i128::try_from(digit).expect("a small index");
            if magnitude > i128::from(u64::MAX) {
                return Err(does_not_fit("fromBase"));
            }
        }
        let signed = if negative { -magnitude } else { magnitude };
        i64::try_from(signed)
            .map(Value::int)
            .map_err(|_| does_not_fit("fromBase"))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Math::format(int|float $n, {decimals?: uint, decimalSeparator?:
    /// string, groupSeparator?: string}): string` — `$n` written for a reader,
    /// replacing PHP's `number_format`.
    ///
    /// Rounding is **half away from zero**, as `number_format`'s is; a caller
    /// who wants another rule rounds first, which is what makes
    /// [`nvs_core_math_round`]'s `mode` reach this member without a second
    /// option here. A non-finite `$n` throws — there is no digit string for an
    /// infinity, and PHP's `"inf"` is not a number a reader can act on.
    ///
    /// The decimal count is capped at [`MAX_DECIMALS`], which is past every
    /// digit an `f64` distinguishes; the cap is what keeps the result's size
    /// a function of the *value* rather than of an argument.
    fn nvs_core_math_format(_ctx, args: [4]) {
        let number = number_at(args, 0, "format")?;
        let decimals = uint_at(args, 1, "format")?;
        let decimal_separator = str_at(args, 2, "format")?;
        let group_separator = str_at(args, 3, "format")?;
        if decimals > MAX_DECIMALS {
            return Err(Fault::thrown(format!(
                "Core\\Math::format was asked for {decimals} decimals, past its cap of \
                 {MAX_DECIMALS}"
            )));
        }
        let decimals = usize::try_from(decimals).expect("a value at or under the cap is small");
        let (sign, integer, fraction) = digits_of(number, decimals)?;
        let mut out = Vec::new();
        if sign {
            out.push(b'-');
        }
        group_into(&integer, group_separator, &mut out);
        if !fraction.is_empty() {
            out.extend_from_slice(decimal_separator);
            out.extend_from_slice(fraction.as_bytes());
        }
        Ok(Value::str(NvsStr::new(&out)))
    }
}

// ============================================================================
// The shared parts
// ============================================================================

/// The most decimals [`nvs_core_math_format`] will write.
///
/// An `f64` distinguishes at most seventeen significant digits, and the
/// smallest subnormal has 1074 places, so nothing past this is information the
/// value holds — it is zeroes, sized by an argument rather than by the number.
const MAX_DECIMALS: u64 = 100;

/// The digit alphabet [`nvs_core_math_to_base`] writes and
/// [`nvs_core_math_from_base`] reads, lowest first, so a digit's value is its
/// own index.
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// `$base` as a radix this module can write in, or the throw an unusable one
/// is.
///
/// Two is the smallest positional base and thirty-six is as far as the digits
/// and the Latin letters reach — the same range `base_convert` accepts, which
/// answers `0` outside it where this throws.
fn radix(base: u64, member: &str) -> Result<u64, Fault> {
    if (2..=36).contains(&base) {
        Ok(base)
    } else {
        Err(Fault::thrown(format!(
            "Core\\Math::{member} has no digits for base {base}: a base runs from 2 to 36"
        )))
    }
}

/// One offending byte as an error message can name it: the character itself
/// where that is printable ASCII, and the byte's own value where it is not — a
/// digit outside the base is far more often a typo than a stray byte, and `y`
/// says which one where `121` does not.
fn readable(byte: u8) -> String {
    if byte.is_ascii_graphic() {
        format!("`{}`", char::from(byte))
    } else {
        format!("byte {byte}")
    }
}

/// The throw an exact answer that does not fit `int` is — the shape `rule:types/arithmetic` makes an overflow, `ArithmeticError`, rather than PHP's silent widening
/// to `float`.
fn does_not_fit(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Arithmetic,
        format!("Core\\Math::{member}'s answer does not fit an `int`"),
    )
}

/// The throw [`nvs_core_math_int_div`]'s two failures share, naming which one
/// it was — the divisor is the only thing that tells them apart.
fn divide_by_zero_or_overflow(member: &str, divisor: i64) -> Fault {
    if divisor == 0 {
        Fault::thrown_as(
            ThrownClass::Arithmetic,
            format!("Core\\Math::{member} was given a zero divisor"),
        )
    } else {
        does_not_fit(member)
    }
}

/// Euclid's algorithm over two non-negative `i128`s.
fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

/// The argument that compares `wanted` against the other, as a fresh
/// reference — [`nvs_core_math_min`] and [`nvs_core_math_max`] in one
/// comparison. The **first** argument wins a tie.
fn pick(
    ctx: &mut Ctx,
    args: &[Value],
    wanted: std::cmp::Ordering,
    member: &str,
) -> Result<Value, Fault> {
    let chosen = if compare_values(ctx, &args[1], &args[0], member)? == wanted {
        args[1]
    } else {
        args[0]
    };
    Ok(owned(chosen))
}

/// One borrowed argument handed back as this member's result, with a
/// reference of its own.
///
/// A `Core` member borrows its arguments and returns one fresh reference
/// (see [`crate`]'s own docs), so a member that answers *with* one of its
/// arguments — `min`, `max`, `clamp` — is the one shape that has to retain.
fn owned(value: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the argument is owned by the caller's frame, which outlives \
                  this call, so the value handed back needs a reference of its \
                  own"
    )]
    unsafe {
        value.retain();
    }
    value
}

/// Spec § 3's six rounding rules, decoded from the `mode` option's integer.
///
/// `pub(crate)` because `Core\RoundMode` is a `Core` enum rather than this
/// class's own — [`ROUND_MODE`]'s own doc says why it is flat — and
/// [`crate::decimal`]'s `divRound` is the second member to take one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoundMode {
    /// A tie goes away from zero — PHP's `PHP_ROUND_HALF_UP`, and `round`'s
    /// default there and here.
    HalfUp,
    /// A tie goes toward zero — `PHP_ROUND_HALF_DOWN`.
    HalfDown,
    /// A tie goes to the even neighbour — `PHP_ROUND_HALF_EVEN`, "banker's
    /// rounding," the rule that does not accumulate a bias over many values.
    HalfEven,
    /// A tie goes to the odd neighbour — `PHP_ROUND_HALF_ODD`.
    HalfOdd,
    /// **Every** value goes away from zero, tie or not — `ceil` for a positive
    /// number and `floor` for a negative one, at any precision.
    Up,
    /// Every value goes toward zero — truncation, at any precision.
    Down,
}

/// The `mode` option at `index`, or the `FATAL` a value that is no case is.
///
/// `member` is the caller's own spelling, because the two members that take a
/// `Core\RoundMode` are in two classes and a `FATAL` names the one that got
/// the value.
pub(crate) fn round_mode(args: &[Value], index: usize, member: &str) -> Result<RoundMode, Fault> {
    match args[index].as_int() {
        Some(0) => Ok(RoundMode::HalfUp),
        Some(1) => Ok(RoundMode::HalfDown),
        Some(2) => Ok(RoundMode::HalfEven),
        Some(3) => Ok(RoundMode::HalfOdd),
        Some(4) => Ok(RoundMode::Up),
        Some(5) => Ok(RoundMode::Down),
        // Unreachable from source: every parameter that reaches here is
        // declared `CoreTy::Enum(r"Core\RoundMode")`, so anything else is
        // `E0401: expected `Core\RoundMode`, found `int`` at the argument,
        // and the enum has exactly the six cases matched above — the arms are
        // its whole roster, not a prefix of it, so a case cannot arrive here
        // either.
        _ => Err(Fault::fatal(format!(
            "{member} expected a `Core\\RoundMode` case for `mode`, got tag {} value {}",
            args[index].tag_byte(),
            args[index].bits()
        ))),
    }
}

/// `$n` rounded to `precision` decimal places under `mode`.
///
/// A positive precision keeps that many places, zero rounds to an integer, and
/// a negative one rounds to tens, hundreds and up — PHP's `round($n, -2)`.
///
/// The scale-round-unscale walk is the same one every language's `round` does,
/// and it inherits the same limitation: `10 ** $precision` is exact only up to
/// twenty-two, and a scaled value past 2^53 has no fractional part left to
/// decide, so both cases answer `$n` unchanged rather than a value the
/// scaling invented.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the exponent is bounded to ±22 immediately above the cast"
)]
fn round_to(value: f64, precision: i64, mode: RoundMode) -> f64 {
    /// Past this the scale itself is inexact, so scaling would decide the
    /// answer rather than the value would.
    const MAX_EXACT_EXPONENT: i64 = 22;

    if !value.is_finite() || precision.abs() > MAX_EXACT_EXPONENT {
        return value;
    }
    let scale = 10f64.powi(precision as i32);
    let scaled = value * scale;
    if scaled.abs() >= (1u64 << 53) as f64 {
        return value;
    }
    apply(scaled, mode) / scale
}

/// One `mode` applied to an already-scaled value, so [`round_to`] holds the
/// scaling and this holds the six rules.
fn apply(scaled: f64, mode: RoundMode) -> f64 {
    let is_tie = (scaled.fract().abs() - 0.5).abs() < f64::EPSILON;
    match mode {
        RoundMode::Up => {
            if scaled < 0.0 {
                scaled.floor()
            } else {
                scaled.ceil()
            }
        }
        RoundMode::Down => scaled.trunc(),
        // `f64::round` is already half-away-from-zero.
        RoundMode::HalfUp => scaled.round(),
        RoundMode::HalfDown if is_tie => scaled.trunc(),
        RoundMode::HalfDown => scaled.round(),
        RoundMode::HalfEven => scaled.round_ties_even(),
        RoundMode::HalfOdd if is_tie => {
            // Whichever of the two neighbours is odd. `rem_euclid` rather than
            // `%` so a negative lower neighbour answers about its parity and
            // not about its sign.
            let lower = scaled.floor();
            if lower.rem_euclid(2.0) == 1.0 {
                lower
            } else {
                scaled.ceil()
            }
        }
        RoundMode::HalfOdd => scaled.round(),
    }
}

/// `$n` as [`nvs_core_math_format`]'s three pieces: whether it is negative,
/// its integer digits, and its `decimals` fractional digits (empty for none).
///
/// The rounding happens *here*, before the digits are written, so the value
/// `{:.*}` formats is already at the wanted precision — Rust's own formatter
/// breaks a tie to even, and `number_format` breaks it away from zero.
fn digits_of(number: Number, decimals: usize) -> Result<(bool, String, String), Fault> {
    let written = match number {
        Number::Integer(n) => {
            let mut text = n.unsigned_abs().to_string();
            if decimals > 0 {
                text.push('.');
                text.extend(std::iter::repeat_n('0', decimals));
            }
            return Ok((n < 0, split_at_point(&text).0, split_at_point(&text).1));
        }
        Number::Real(n) => {
            if !n.is_finite() {
                return Err(Fault::thrown(
                    "Core\\Math::format has no digits for an infinity or a `NaN`".to_owned(),
                ));
            }
            let rounded = round_to(
                n,
                i64::try_from(decimals).expect("at or under the cap"),
                RoundMode::HalfUp,
            );
            format!("{rounded:.decimals$}")
        }
        // Rounded over the digits themselves rather than over the value: a
        // `decimal` is exact, so rounding it through any numeric intermediate
        // would be the one place this member lost the precision `rule:types/decimal`
        // exists to keep. It also lifts the `f64` path's ceiling — `decimals`
        // may be anything up to `MAX_DECIMALS`, where a `decimal`'s own scale
        // stops at 28.
        Number::Exact(n) => return Ok(decimal_digits(n, decimals)),
    };
    let negative = written.starts_with('-');
    let magnitude = written.strip_prefix('-').unwrap_or(&written);
    let (integer, fraction) = split_at_point(magnitude);
    Ok((negative, integer, fraction))
}

/// One `decimal`'s digits, rounded **half up** at `decimals` places — the
/// exact counterpart of the `f64` arm's `round_to` plus `{:.n}`, and the same
/// rounding mode, so the two arms of [`digits_of`] agree on a value both can
/// hold.
///
/// Rounding is done on the digit string, which is what keeps it exact and
/// total: no intermediate can overflow 96 bits, and a `decimals` past
/// `decimal`'s own scale of 28 simply pads.
fn decimal_digits(value: Decimal, decimals: usize) -> (bool, String, String) {
    let text = value.abs().to_string();
    let (integer, fraction) = split_at_point(&text);
    if fraction.len() <= decimals {
        let mut fraction = fraction;
        fraction.extend(std::iter::repeat_n('0', decimals - fraction.len()));
        return (value.is_negative(), integer, fraction);
    }
    let mut digits = format!("{integer}{}", &fraction[..decimals]).into_bytes();
    if fraction.as_bytes()[decimals] >= b'5' {
        carry_one(&mut digits);
    }
    let kept = String::from_utf8(digits).expect("every byte written above is an ASCII digit");
    let split = kept.len() - decimals;
    (
        value.is_negative(),
        kept[..split].to_owned(),
        kept[split..].to_owned(),
    )
}

/// Adds one to a big-endian run of ASCII digits in place, growing it by a
/// leading `1` where every digit carried (`999` becomes `1000`).
fn carry_one(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

/// A written magnitude split at its decimal point, with an empty fraction for
/// one that has none.
fn split_at_point(text: &str) -> (String, String) {
    match text.split_once('.') {
        Some((integer, fraction)) => (integer.to_owned(), fraction.to_owned()),
        None => (text.to_owned(), String::new()),
    }
}

/// `digits` written into `out` with `separator` between each group of three,
/// counted from the right — an empty separator writes the digits unchanged.
fn group_into(digits: &str, separator: &[u8], out: &mut Vec<u8>) {
    let bytes = digits.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        let remaining = bytes.len() - index;
        if index > 0 && remaining.is_multiple_of(3) {
            out.extend_from_slice(separator);
        }
        out.push(*byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, call};

    /// A member end to end through the `rule:errors/propagation` boundary compiled code will
    /// reach it at, as `crate::arr`'s own tests do.
    fn float_result(function: nvs_runtime::NvsFn, args: &[Value]) -> f64 {
        let mut ctx = Ctx::buffered();
        call(function, &mut ctx, args)
            .expect("the call succeeds")
            .as_float()
            .expect("a float result")
    }

    fn text_result(function: nvs_runtime::NvsFn, args: &[Value]) -> String {
        let mut ctx = Ctx::buffered();
        let value = call(function, &mut ctx, args).expect("the call succeeds");
        let text = String::from_utf8(value.as_str_bytes().expect("a string result").to_vec())
            .expect("`rule:types/bytes` makes a string UTF-8");
        #[expect(
            unsafe_code,
            reason = "the result carries the one reference this test owns"
        )]
        unsafe {
            value.release();
        }
        text
    }

    /// Spec § 3's six modes, on the one value that tells all of them apart.
    #[test]
    fn every_round_mode_breaks_the_tie_its_own_way() {
        let cases = [
            (0, 3.0, -3.0), // HalfUp
            (1, 2.0, -2.0), // HalfDown
            (2, 2.0, -2.0), // HalfEven
            (3, 3.0, -3.0), // HalfOdd
            (4, 3.0, -3.0), // Up
            (5, 2.0, -2.0), // Down
        ];
        for (mode, positive, negative) in cases {
            let options = [Value::int(0), Value::int(mode)];
            assert_eq!(
                float_result(
                    nvs_core_math_round,
                    &[Value::float(2.5), options[0], options[1]]
                ),
                positive,
                "mode {mode} over 2.5"
            );
            assert_eq!(
                float_result(
                    nvs_core_math_round,
                    &[Value::float(-2.5), options[0], options[1]]
                ),
                negative,
                "mode {mode} over -2.5"
            );
        }
    }

    /// A precision moves the tie, in both directions.
    #[test]
    fn a_precision_rounds_at_that_place() {
        let half_up = [Value::int(2), Value::int(0)];
        assert_eq!(
            float_result(
                nvs_core_math_round,
                &[Value::float(1.005), half_up[0], half_up[1]]
            ),
            1.0
        );
        let tens = [Value::int(-2), Value::int(0)];
        assert_eq!(
            float_result(
                nvs_core_math_round,
                &[Value::float(1250.0), tens[0], tens[1]]
            ),
            1300.0
        );
    }

    /// The grouping walk, at every length a three-digit group can end on.
    #[test]
    fn a_group_separator_lands_every_three_digits_from_the_right() {
        let comma = Value::str(NvsStr::new(b","));
        let point = Value::str(NvsStr::new(b"."));
        for (value, want) in [
            (1.0, "1"),
            (12.0, "12"),
            (123.0, "123"),
            (1234.0, "1,234"),
            (1_234_567.0, "1,234,567"),
        ] {
            assert_eq!(
                text_result(
                    nvs_core_math_format,
                    &[Value::float(value), Value::uint(0), point, comma]
                ),
                want
            );
        }
        #[expect(
            unsafe_code,
            reason = "each literal carries the one reference this test owns"
        )]
        unsafe {
            comma.release();
            point.release();
        }
    }

    /// The two base members are inverses, including across the sign.
    #[test]
    fn a_base_written_reads_back() {
        for value in [0i64, 7, -7, 48879, i64::MAX, i64::MIN] {
            for base in [2u64, 8, 16, 36] {
                let written = text_result(
                    nvs_core_math_to_base,
                    &[Value::int(value), Value::uint(base)],
                );
                let text = Value::str(NvsStr::new(written.as_bytes()));
                let mut ctx = Ctx::buffered();
                let read = call(
                    nvs_core_math_from_base,
                    &mut ctx,
                    &[text, Value::uint(base)],
                )
                .expect("the call succeeds");
                #[expect(
                    unsafe_code,
                    reason = "the literal carries the one reference this test owns"
                )]
                unsafe {
                    text.release();
                }
                assert_eq!(read.as_int(), Some(value), "{value} in base {base}");
            }
        }
    }
}
