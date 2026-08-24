//! `Core\Time\Duration` —
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 4's exact-offset half, and the type
//! [ADR 0070](../../../../docs/adr/0070-duration-literals.md)'s `30s`/`1h30m`
//! literal is.
//!
//! # One state, one grammar, no dependency
//!
//! A `Duration` is exactly one `int` slot holding **nanoseconds**, so its
//! arithmetic is `i64` arithmetic and this module needs no outside crate at
//! all — `jiff`, which `docs/agent/loop-goal.md` names for the rest of § 4,
//! answers calendar questions that an exact offset does not have. `Instant`,
//! `DateTime` and `Zone` are still owed and are where that dependency lands;
//! gap 1 below is the list.
//!
//! The grammar `parse` accepts and `toString` emits is **not here**: it is
//! [`mwl_syntax::duration`], which the lexer calls for the source literal and
//! `mwl.toml` will call for a duration-valued directive. ADR 0070 § 5 requires
//! the three to share one implementation, and that module's own docs own why
//! it sits in the syntax crate rather than this one.
//!
//! # What a duration spends
//!
//! One [`crate::instance`] object per value — a header plus one 16-byte slot,
//! charged to the request that produced it. A literal spends it too: ADR 0070
//! § 3's constant-pool folding wants an *immortal* value with no allocation at
//! all, which is the same thing `mwl_runtime`'s own gap 3 owes a string
//! literal, so both close together rather than one growing a mechanism the
//! other does not use. Until then `30s` is one [`mwl_core_time_duration_nanoseconds`]
//! call on a folded constant, which is already the whole grammar resolved at
//! compile time.
//!
//! # Known gaps
//!
//! 1. **§ 4's other three types are not registered.** `Core\Time` itself
//!    (`now`, `monotonic`, `sleep`, `fromEpoch`, `fromIso`, `parse`, `at`),
//!    `Instant`, `DateTime`, `Zone`, `Date` and `TimeOfDay`, plus the
//!    `Weekday`/`Month`/`Unit` enums, all wait on `jiff` and on the CLDR
//!    pattern half of `format`. `examples/dates.mwl` is the fixture that
//!    fails until they exist.
//! 2. **`Comparable` and `Stringable` are satisfied by member, not by
//!    declaration.** The spec says a `Duration` implements both; `compareTo`
//!    and `toString` are registered and behave exactly as those interfaces
//!    require, but `mwl_types`' reserved interfaces carry no member signatures
//!    yet, so `$a < $b` and `"took " . $d` are both still refused — `mwl-ir`'s
//!    gap 12 owns the second. `$d->compareTo($e)` and `$d->toString()` are the
//!    spellings that work today.

use mwl_runtime::{Fault, MwlStr, Value};
use mwl_syntax::duration;

use crate::registry::{CoreClass, CoreMethod, CoreTy};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Time\Duration`'s fully-qualified name, written once — [`DURATION`]
/// declares it and every [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
///
/// `mwl_types::expr` reads this same constant for the type of an
/// [`mwl_syntax::ast::ExprKind::Duration`] literal, which is what makes ADR
/// 0070 § 2's "the suffix *is* the type" one fact rather than two spellings.
pub const DURATION_NAME: &str = r"Core\Time\Duration";

/// The symbol behind `Core\Time\Duration::nanoseconds`, which is also how a
/// duration **literal** reaches a value — `mwl_ir::lower` emits one
/// `InstKind::CoreCall` to it with the folded nanosecond count.
///
/// Named here rather than spelled in `mwl-ir` so that the registry row and the
/// literal cannot come to mean different things.
pub const FROM_NANOS_SYMBOL: &str = "mwl_core_time_duration_nanoseconds";

/// Spec § 4's `Core\Time\Duration` — an exact count of nanoseconds, and the
/// one arithmetic that a DST boundary or a short month cannot change the
/// length of.
///
/// The eight `Duration::seconds`-shaped constructors are for a **computed**
/// count; a constant one is ADR 0070's literal, and the two produce the same
/// value through this class's one slot.
pub const DURATION: CoreClass = CoreClass {
    name: DURATION_NAME,
    methods: &[
        CoreMethod {
            name: "nanoseconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: FROM_NANOS_SYMBOL,
        },
        CoreMethod {
            name: "microseconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_microseconds",
        },
        CoreMethod {
            name: "milliseconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_milliseconds",
        },
        CoreMethod {
            name: "seconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_seconds",
        },
        CoreMethod {
            name: "minutes",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_minutes",
        },
        CoreMethod {
            name: "hours",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_hours",
        },
        CoreMethod {
            name: "days",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_days",
        },
        CoreMethod {
            name: "weeks",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_weeks",
        },
        CoreMethod {
            name: "parse",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_parse",
        },
    ],
    instance: &[
        CoreMethod {
            name: "toNanoseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_duration_to_nanoseconds",
        },
        CoreMethod {
            name: "toMicroseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_duration_to_microseconds",
        },
        CoreMethod {
            name: "toMilliseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_duration_to_milliseconds",
        },
        CoreMethod {
            name: "toSeconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_duration_to_seconds",
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_plus",
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_minus",
        },
        CoreMethod {
            name: "multipliedBy",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_multiplied_by",
        },
        CoreMethod {
            name: "negated",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_duration_negated",
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_duration_compare_to",
        },
        CoreMethod {
            name: "toString",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_time_duration_to_string",
        },
    ],
    slots: &["nanos"],
    constants: &[],
};

/// [`DURATION`]'s one slot, by index — see `crate::regex`'s own slot constants
/// for why this is a constant rather than a lookup per call.
const NANOS_SLOT: usize = 0;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        FROM_NANOS_SYMBOL => (mwl_core_time_duration_nanoseconds as *const ()).cast(),
        "mwl_core_time_duration_microseconds" => {
            (mwl_core_time_duration_microseconds as *const ()).cast()
        }
        "mwl_core_time_duration_milliseconds" => {
            (mwl_core_time_duration_milliseconds as *const ()).cast()
        }
        "mwl_core_time_duration_seconds" => (mwl_core_time_duration_seconds as *const ()).cast(),
        "mwl_core_time_duration_minutes" => (mwl_core_time_duration_minutes as *const ()).cast(),
        "mwl_core_time_duration_hours" => (mwl_core_time_duration_hours as *const ()).cast(),
        "mwl_core_time_duration_days" => (mwl_core_time_duration_days as *const ()).cast(),
        "mwl_core_time_duration_weeks" => (mwl_core_time_duration_weeks as *const ()).cast(),
        "mwl_core_time_duration_parse" => (mwl_core_time_duration_parse as *const ()).cast(),
        "mwl_core_time_duration_to_nanoseconds" => {
            (mwl_core_time_duration_to_nanoseconds as *const ()).cast()
        }
        "mwl_core_time_duration_to_microseconds" => {
            (mwl_core_time_duration_to_microseconds as *const ()).cast()
        }
        "mwl_core_time_duration_to_milliseconds" => {
            (mwl_core_time_duration_to_milliseconds as *const ()).cast()
        }
        "mwl_core_time_duration_to_seconds" => {
            (mwl_core_time_duration_to_seconds as *const ()).cast()
        }
        "mwl_core_time_duration_plus" => (mwl_core_time_duration_plus as *const ()).cast(),
        "mwl_core_time_duration_minus" => (mwl_core_time_duration_minus as *const ()).cast(),
        "mwl_core_time_duration_multiplied_by" => {
            (mwl_core_time_duration_multiplied_by as *const ()).cast()
        }
        "mwl_core_time_duration_negated" => (mwl_core_time_duration_negated as *const ()).cast(),
        "mwl_core_time_duration_compare_to" => {
            (mwl_core_time_duration_compare_to as *const ()).cast()
        }
        "mwl_core_time_duration_to_string" => {
            (mwl_core_time_duration_to_string as *const ()).cast()
        }
        _ => return None,
    })
}

// ============================================================================
// Shared reading and building
// ============================================================================

/// A fresh `Duration` holding `nanos`.
fn built(nanos: i64) -> Value {
    crate::instance::build(&DURATION, [Value::int(nanos)])
}

/// The `int` in argument slot `at`, for a constructor's count or
/// `multipliedBy`'s factor.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member — `mwl_types` has already checked the
/// declared type, so a wrong tag here is compiled code disagreeing with the
/// registry rather than anything a program can write.
fn count(args: &[Value], at: usize, member: &str) -> Result<i64, Fault> {
    args[at].as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Time\\Duration::{member} expected an `int`, got tag {}",
            args[at].tag_byte()
        ))
    })
}

/// The nanosecond count of the `Duration` in argument slot `at` — slot 0 for a
/// receiver, any other slot for a `Duration` *parameter*.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives.
fn nanos_of(args: &[Value], at: usize, member: &str) -> Result<i64, Fault> {
    let object = crate::instance::receiver(args[at], &DURATION, member)?;
    crate::instance::slot(object, NANOS_SLOT)
        .as_int()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Duration::{member} found a non-`int` `nanos` slot"
            ))
        })
}

/// ADR 0063 R4: a `Duration` past `i64` nanoseconds throws rather than wraps,
/// which is [`duration::DurationError::Overflow`]'s message at run time and
/// the same sentence the lexer prints for a literal.
fn overflowed(member: &str) -> Fault {
    Fault::thrown(format!(
        "Core\\Time\\Duration::{member}(): {}",
        duration::DurationError::Overflow.message()
    ))
}

/// A constructor over one unit: `count` of `length`-nanosecond units.
fn scaled(args: &[Value], member: &str, length: i64) -> Result<Value, Fault> {
    let n = count(args, 0, member)?;
    n.checked_mul(length)
        .map(built)
        .ok_or_else(|| overflowed(member))
}

// ============================================================================
// The eight constructors, plus `parse`
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::nanoseconds(int $n): Duration`.
    ///
    /// Also where a **literal** lands: ADR 0070 § 3 folds `1h30m` to its
    /// nanosecond count while compiling, and `mwl-ir` emits one call to this
    /// with that constant — so the literal and the constructor cannot produce
    /// different values.
    fn mwl_core_time_duration_nanoseconds(_ctx, args: [1]) {
        Ok(built(count(args, 0, "nanoseconds")?))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::microseconds(int $n): Duration`.
    fn mwl_core_time_duration_microseconds(_ctx, args: [1]) {
        scaled(args, "microseconds", 1_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::milliseconds(int $n): Duration`.
    fn mwl_core_time_duration_milliseconds(_ctx, args: [1]) {
        scaled(args, "milliseconds", 1_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::seconds(int $n): Duration`.
    fn mwl_core_time_duration_seconds(_ctx, args: [1]) {
        scaled(args, "seconds", 1_000_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::minutes(int $n): Duration`.
    fn mwl_core_time_duration_minutes(_ctx, args: [1]) {
        scaled(args, "minutes", 60 * 1_000_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::hours(int $n): Duration`.
    fn mwl_core_time_duration_hours(_ctx, args: [1]) {
        scaled(args, "hours", 60 * 60 * 1_000_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::days(int $n): Duration` — exactly 24 hours per
    /// day, never a calendar day (ADR 0070 § 1). A calendar step is
    /// `DateTime::plus($n, Unit::Day)`, which is a different type's member for
    /// exactly this reason.
    fn mwl_core_time_duration_days(_ctx, args: [1]) {
        scaled(args, "days", 24 * 60 * 60 * 1_000_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::weeks(int $n): Duration` — exactly 168 hours, on
    /// the same reasoning as `days`.
    fn mwl_core_time_duration_weeks(_ctx, args: [1]) {
        scaled(args, "weeks", 7 * 24 * 60 * 60 * 1_000_000_000)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Duration::parse(string $text): Duration` — ADR 0070 § 5's
    /// run-time entry point into the *same* grammar the lexer reads, so
    /// `Duration::parse("1h30m")` and the literal `1h30m` are one value.
    ///
    /// Throws on anything it does not accept, which is what makes it an
    /// [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
    /// launderer for a `tainted` config string — the qualifier half of that is
    /// still owed, and `crate::regex`'s gap 2 owns why nothing in
    /// [`crate::registry`] can state it yet.
    fn mwl_core_time_duration_parse(_ctx, args: [1]) {
        let bytes = args[0].as_str_bytes().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Duration::parse expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // A `string` is guaranteed-valid UTF-8 (ADR 0009), so this cannot
        // fail for anything compiled code produced.
        let text = std::str::from_utf8(bytes)
            .map_err(|_| Fault::fatal("Core\\Time\\Duration::parse got invalid UTF-8"))?;
        match duration::parse(text) {
            Ok(nanos) => Ok(built(nanos)),
            Err(err) => Err(Fault::thrown(format!(
                "Core\\Time\\Duration::parse(): {}",
                err.message()
            ))),
        }
    }
}

// ============================================================================
// The instance members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `$d->toNanoseconds(): int` — the whole state, exactly.
    fn mwl_core_time_duration_to_nanoseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toNanoseconds")?))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->toMicroseconds(): int`, truncating toward zero — `1500ns` is `1`
    /// and `-1500ns` is `-1`, which is what Rust's and PHP's integer division
    /// both already do, so nothing new has to be remembered.
    fn mwl_core_time_duration_to_microseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toMicroseconds")? / 1_000))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->toMilliseconds(): int`, truncating toward zero.
    fn mwl_core_time_duration_to_milliseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toMilliseconds")? / 1_000_000))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->toSeconds(): int`, truncating toward zero.
    fn mwl_core_time_duration_to_seconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toSeconds")? / 1_000_000_000))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->plus(Duration $other): Duration` — a fresh value, since ADR 0063
    /// R3 makes every `Core` member pure.
    fn mwl_core_time_duration_plus(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "plus")?;
        let right = nanos_of(args, 1, "plus")?;
        left.checked_add(right)
            .map(built)
            .ok_or_else(|| overflowed("plus"))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->minus(Duration $other): Duration` — the spelling ADR 0070 § 4
    /// gives a backwards step, since `-7d` does not parse.
    fn mwl_core_time_duration_minus(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "minus")?;
        let right = nanos_of(args, 1, "minus")?;
        left.checked_sub(right)
            .map(built)
            .ok_or_else(|| overflowed("minus"))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->multipliedBy(int $factor): Duration`.
    fn mwl_core_time_duration_multiplied_by(_ctx, args: [2]) {
        let nanos = nanos_of(args, 0, "multipliedBy")?;
        let factor = count(args, 1, "multipliedBy")?;
        nanos
            .checked_mul(factor)
            .map(built)
            .ok_or_else(|| overflowed("multipliedBy"))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->negated(): Duration`.
    ///
    /// The single value with no negation is `nanoseconds(i64::MIN)`, which
    /// throws rather than wrapping to itself — the one place this member can
    /// fail, and the reason it is written with `checked_neg` rather than `-`.
    fn mwl_core_time_duration_negated(_ctx, args: [1]) {
        nanos_of(args, 0, "negated")?
            .checked_neg()
            .map(built)
            .ok_or_else(|| overflowed("negated"))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->compareTo(Duration $other): int` — `Comparable`'s member
    /// ([ADR 0013](../../../../docs/adr/0013-comparable-interface.md)),
    /// answering the sign of `$d - $other` without the subtraction's overflow.
    fn mwl_core_time_duration_compare_to(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "compareTo")?;
        let right = nanos_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->toString(): string` — `Stringable`'s member
    /// ([ADR 0028](../../../../docs/adr/0028-closing-the-remaining-magic-methods.md)),
    /// emitting ADR 0070 § 1's grammar so that a value round-trips through
    /// `parse`.
    fn mwl_core_time_duration_to_string(_ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "toString")?;
        Ok(Value::str(MwlStr::new(duration::render(nanos).as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slot constant and the registered layout agree — the one thing a
    /// paste error here would turn into a silent wrong read.
    #[test]
    fn the_slot_constant_matches_the_registered_layout() {
        assert_eq!(NANOS_SLOT, DURATION.slot("nanos"));
        assert_eq!(DURATION.slots.len(), 1);
    }

    /// Every symbol this class registers resolves in this module's own
    /// `address` — the miss that is a *runtime* panic rather than a link
    /// error, so it is worth a test of its own.
    #[test]
    fn every_registered_symbol_has_an_address_here() {
        for method in DURATION.members() {
            assert!(
                address(method.symbol).is_some(),
                "{} has no address",
                method.symbol
            );
        }
    }
}
