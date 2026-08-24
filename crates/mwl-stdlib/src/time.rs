//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 4 — `Core\Time` and the types it answers with, one module because they
//! are one domain: `Duration` (the type
//! [ADR 0070](../../../../docs/adr/0070-duration-literals.md)'s `30s`/`1h30m`
//! literal is), `Instant`, `Zone` and `DateTime`. The pattern grammar
//! `DateTime::format` and `Time::parse` share is [`crate::cldr`], which is a
//! grammar of its own and so a module of its own.
//!
//! # Two arithmetics, and the type says which one
//!
//! § 4's opening rule, restated here only because it is what splits this
//! module into the parts below: an `Instant` moves by a `Duration`, an exact
//! count of nanoseconds that nothing can lengthen; a `DateTime` moves by a
//! count of a `Unit`, a calendar step a DST boundary or a short month can make
//! longer or shorter. So `Duration` needs no calendar at all and is written
//! over `i64`, while everything that converts between an instant and a
//! calendar goes through [`jiff`] and an explicit `Zone` — there is no ambient
//! timezone anywhere in this module, and no member takes one implicitly.
//!
//! # Decision: `jiff`, for the type set rather than the API
//!
//! `docs/agent/loop-goal.md` names it, and what it buys is that its own types
//! map one-to-one onto the ones § 4 already wrote: [`jiff::Timestamp`] is an
//! `Instant`, [`jiff::tz::TimeZone`] is a `Zone`, and its `Zoned`/`civil`
//! split is the same absolute-versus-civil distinction the spec is built on —
//! so nothing here has to invent a bridging concept. It is pure Rust with no
//! build script, and on a platform with no IANA database (Windows) it embeds
//! one rather than shelling out, which is what makes `Zone::of` answer the
//! same on both of this repository's two test legs. The alternative with the
//! same properties, `time`, has no time-zone database at all.
//!
//! **What it spends:** the bundled `jiff-tzdb-platform` database is a few
//! hundred kilobytes of the binary on Windows, once, and a `TimeZone` looked
//! up by name is cached by `jiff` itself rather than re-parsed per call.
//!
//! # One grammar for a duration, and it is not here
//!
//! The grammar `Duration::parse` accepts and `toString` emits is
//! [`mwl_syntax::duration`], which the lexer calls for the source literal and
//! `mwl.toml` will call for a duration-valued directive. ADR 0070 § 5 requires
//! the three to share one implementation, and that module's own docs own why
//! it sits in the syntax crate rather than this one.
//!
//! # What a value here spends
//!
//! One [`crate::instance`] object per value — a header plus 16 bytes per
//! declared slot, charged to the request that produced it. A `Duration` and a
//! `Zone` hold one slot each; an `Instant` holds two, its epoch second and its
//! subsecond nanosecond, which is [`jiff::Timestamp`]'s own state and gives the
//! full ±9999-year range rather than the ~1677-2262 one a single `i64` of
//! nanoseconds would; a `DateTime` holds three, an `Instant`'s two plus a
//! `Zone`'s one, and [`DATETIME`] owns why that rather than seven civil
//! fields. A duration *literal* spends an allocation too: ADR 0070
//! § 3's constant-pool folding wants an *immortal* value with no allocation at
//! all, which is the same thing `mwl_runtime`'s own gap 3 owes a string
//! literal, so both close together rather than one growing a mechanism the
//! other does not use. Until then `30s` is one
//! [`mwl_core_time_duration_nanoseconds`] call on a folded constant, which is
//! already the whole grammar resolved at compile time.
//!
//! # Known gaps
//!
//! 1. **`Core\Time\Date` and `Core\Time\TimeOfDay` are not registered**,
//!    and with them the three [`DATETIME`] members that produce or take one:
//!    `date()`, `timeOfDay()` and `withTime()`. They are § 4's zone-free
//!    component types, with the same `plus`/`minus`/`with`/`compareTo`/`format`
//!    shape over [`jiff::civil::Date`]/[`jiff::civil::Time`] and the
//!    constructors `Date::at` and `TimeOfDay::at` — a second and third class
//!    over machinery that now exists, rather than anything new. § 4's
//!    `Core\Month` enum waits with them: nothing takes or answers with one
//!    until `Date` does, and [`crate::registry::ENUMS`] states that rule.
//! 2. **`$d->format` and `Core\Time::parse` compile their pattern per call.**
//!    ADR 0057 makes both intrinsics whose literal pattern is prepared while
//!    compiling; [`crate::cldr`]'s own gap 1 owns what that changes and what
//!    it does not.
//! 3. **`Core\Time::sleep` blocks the core thread.** M5's scheduler is what
//!    turns it into a suspension point; until then it is
//!    [`std::thread::sleep`], which is correct for a CLI program and wrong for
//!    a request. The signature does not change when that lands.
//! 4. **`Comparable` and `Stringable` are satisfied by member, not by
//!    declaration.** The spec says a `Duration` implements both and an
//!    `Instant` implements `Comparable`; `compareTo` and `toString` are
//!    registered and behave exactly as those interfaces require, but
//!    `mwl_types`' reserved interfaces carry no member signatures yet, so
//!    `$a < $b` and `"took " . $d` are both still refused — `mwl-ir`'s gap 12
//!    owns the second. `$d->compareTo($e)` and `$d->toString()` are the
//!    spellings that work today.

use std::sync::OnceLock;

use jiff::civil::{self, Weekday};
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp, Zoned};
use mwl_runtime::{Fault, MwlStr, Value};
use mwl_syntax::duration;

use crate::registry::{Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy};

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
///
/// One arm per class rather than one match over all four: § 4 is one domain
/// but four classes, and a class's symbols stay beside the class.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    duration_address(symbol)
        .or_else(|| time_address(symbol))
        .or_else(|| instant_address(symbol))
        .or_else(|| datetime_address(symbol))
        .or_else(|| zone_address(symbol))
}

/// [`DURATION`]'s symbols.
fn duration_address(symbol: &str) -> Option<*const u8> {
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
        let text = text_of(args, 0, "Core\\Time\\Duration::parse")?;
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

// ============================================================================
// `Core\Time\Zone` — registration
// ============================================================================

/// `Core\Time\Zone`'s fully-qualified name, written once, for
/// [`DURATION_NAME`]'s reason.
pub const ZONE_NAME: &str = r"Core\Time\Zone";

/// Spec § 4's `Core\Time\Zone` — the thing every instant↔calendar conversion
/// takes explicitly, since § 4 has no ambient timezone at all.
///
/// One `string` slot holding the zone's **id**: an IANA identifier
/// (`"Europe/Berlin"`) for a region, or a `±HH:MM[:SS]` offset spelling for a
/// [`mwl_core_time_zone_fixed`] one. The two cannot collide — no IANA
/// identifier begins with a sign — so [`resolve_zone`] tells them apart by the
/// first byte, and a `Zone` stays a value MWL can hold rather than a native
/// handle ([`crate::instance`] owns why that matters).
pub const ZONE: CoreClass = CoreClass {
    name: ZONE_NAME,
    methods: &[
        CoreMethod {
            name: "of",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "mwl_core_time_zone_of",
        },
        CoreMethod {
            name: "fixed",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "mwl_core_time_zone_fixed",
        },
        CoreMethod {
            name: "system",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "mwl_core_time_zone_system",
        },
    ],
    instance: &[CoreMethod {
        name: "offsetAt",
        params: &[CoreTy::Instance(INSTANT_NAME)],
        defaults: &[],
        return_ty: CoreTy::Instance(DURATION_NAME),
        symbol: "mwl_core_time_zone_offset_at",
    }],
    slots: &["id"],
    constants: &[CoreConst {
        name: "UTC",
        ty: CoreTy::Instance(ZONE_NAME),
        value: Const::Built {
            symbol: "mwl_core_time_zone_of",
            args: &[Const::Str("UTC")],
        },
    }],
};

/// [`ZONE`]'s one slot, by index.
const ZONE_ID_SLOT: usize = 0;

/// [`ZONE`]'s symbols.
fn zone_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_time_zone_of" => (mwl_core_time_zone_of as *const ()).cast(),
        "mwl_core_time_zone_fixed" => (mwl_core_time_zone_fixed as *const ()).cast(),
        "mwl_core_time_zone_system" => (mwl_core_time_zone_system as *const ()).cast(),
        "mwl_core_time_zone_offset_at" => (mwl_core_time_zone_offset_at as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// `Core\Time\Instant` — registration
// ============================================================================

/// `Core\Time\Instant`'s fully-qualified name, written once, for
/// [`DURATION_NAME`]'s reason.
pub const INSTANT_NAME: &str = r"Core\Time\Instant";

/// Spec § 4's `Core\Time\Instant` — an absolute point on the timeline, with no
/// zone and no calendar of its own.
///
/// Two `int` slots, which are exactly [`jiff::Timestamp`]'s own state: the
/// epoch **second** and the **subsecond nanosecond**, always of the same sign.
/// A single `i64` of nanoseconds would have been one slot cheaper and would
/// have capped the type at 1677–2262, which is a range a program can reach by
/// accident (a far-future expiry, a year-9999 sentinel) — this module's docs
/// own that trade.
pub const INSTANT: CoreClass = CoreClass {
    name: INSTANT_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "in",
            params: &[CoreTy::Instance(ZONE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_instant_in",
        },
        CoreMethod {
            name: "toEpochSeconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_instant_to_epoch_seconds",
        },
        CoreMethod {
            name: "toEpochMillis",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_instant_to_epoch_millis",
        },
        CoreMethod {
            name: "toEpochMicros",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_instant_to_epoch_micros",
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_instant_plus",
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_instant_minus",
        },
        CoreMethod {
            name: "since",
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_instant_since",
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_instant_compare_to",
        },
        CoreMethod {
            name: "toIso",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_time_instant_to_iso",
        },
    ],
    slots: &["seconds", "nanos"],
    constants: &[],
};

/// [`INSTANT`]'s slots, by index.
const INSTANT_SECONDS_SLOT: usize = 0;
/// See [`INSTANT_SECONDS_SLOT`].
const INSTANT_NANOS_SLOT: usize = 1;

/// [`INSTANT`]'s symbols.
fn instant_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_time_instant_in" => (mwl_core_time_instant_in as *const ()).cast(),
        "mwl_core_time_instant_to_epoch_seconds" => {
            (mwl_core_time_instant_to_epoch_seconds as *const ()).cast()
        }
        "mwl_core_time_instant_to_epoch_millis" => {
            (mwl_core_time_instant_to_epoch_millis as *const ()).cast()
        }
        "mwl_core_time_instant_to_epoch_micros" => {
            (mwl_core_time_instant_to_epoch_micros as *const ()).cast()
        }
        "mwl_core_time_instant_plus" => (mwl_core_time_instant_plus as *const ()).cast(),
        "mwl_core_time_instant_minus" => (mwl_core_time_instant_minus as *const ()).cast(),
        "mwl_core_time_instant_since" => (mwl_core_time_instant_since as *const ()).cast(),
        "mwl_core_time_instant_compare_to" => {
            (mwl_core_time_instant_compare_to as *const ()).cast()
        }
        "mwl_core_time_instant_to_iso" => (mwl_core_time_instant_to_iso as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// `Core\Time\DateTime` — registration, and § 4's two enums
// ============================================================================

/// `Core\Time\DateTime`'s fully-qualified name, written once, for
/// [`DURATION_NAME`]'s reason.
pub const DATETIME_NAME: &str = r"Core\Time\DateTime";

/// Spec § 4's `Core\Unit` — the calendar step a `DateTime` moves by, which is
/// the half of § 4's two arithmetics a DST boundary or a short month can
/// lengthen.
///
/// Flat under `Core` rather than under `Core\Time`, following `Core\Order`:
/// [`crate::registry::ENUMS`] states the rule and `docs/agent/loop-goal.md`
/// settled the spelling. The values are the CLDR-free ascending order the spec
/// writes them in, smallest first, so a comparison between two of them means
/// what it reads as.
pub const UNIT: CoreEnum = CoreEnum {
    name: r"Core\Unit",
    cases: &[
        ("Nanosecond", 0),
        ("Microsecond", 1),
        ("Millisecond", 2),
        ("Second", 3),
        ("Minute", 4),
        ("Hour", 5),
        ("Day", 6),
        ("Week", 7),
        ("Month", 8),
        ("Quarter", 9),
        ("Year", 10),
    ],
};

/// Spec § 4's `Core\Weekday`, Monday-first — ISO-8601's own order, which is
/// what `date("N")` already answers and what [`crate::cldr`]'s tables index
/// by.
///
/// Zero-based rather than `date("N")`'s one-based count because ADR 0010 makes
/// a case an ordinary integer constant and nothing here reads it as a
/// day number; `$d->weekday()` answers the case, not the index.
pub const WEEKDAY: CoreEnum = CoreEnum {
    name: r"Core\Weekday",
    cases: &[
        ("Monday", 0),
        ("Tuesday", 1),
        ("Wednesday", 2),
        ("Thursday", 3),
        ("Friday", 4),
        ("Saturday", 5),
        ("Sunday", 6),
    ],
};

/// The `{year?, month?, day?, hour?, minute?, second?, nanos?}` bag
/// `$d->with(…)` takes — spec § 4's replacement for `setDate`, `setTime` and
/// `setISODate` at once.
///
/// Every option defaults to [`Const::Null`], which is that variant's own
/// "not given": there is no year, month or hour value that could mean "leave
/// this one alone", so the omitted case has to be a different tag rather than
/// a sentinel number.
const WITH_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "year",
        ty: CoreTy::Int,
        default: Const::Null,
    },
    CoreOption {
        name: "month",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "day",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "hour",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "minute",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "second",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
    CoreOption {
        name: "nanos",
        ty: CoreTy::Uint,
        default: Const::Null,
    },
];

/// Spec § 4's `Core\Time\DateTime` — a civil date and time **in a zone**, and
/// the half of § 4's two arithmetics that counts calendar units.
///
/// Three slots: an [`INSTANT`]'s two, plus a [`ZONE`]'s one. That is
/// [`jiff::Zoned`]'s own state, and it is the representation with no
/// unrepresentable value in it — a civil datetime stored as seven fields could
/// hold 02:30 on a spring-forward morning, which is a time that does not
/// exist, and every member would then have to decide what it meant. Storing
/// the instant instead settles the ambiguity **once**, where the value is
/// built, which is exactly what [`jiff::tz::TimeZone::to_zoned`] is for; the
/// civil fields are derived on read, which is a lookup in the zone's own
/// transition table rather than a parse.
///
/// **What it spends:** 48 bytes of slots against 32 for an `Instant`, and one
/// string reference for the zone id, shared with the `Zone` a conversion was
/// asked for in.
pub const DATETIME: CoreClass = CoreClass {
    name: DATETIME_NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "format",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "mwl_core_time_datetime_format",
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_plus",
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_minus",
        },
        CoreMethod {
            name: "next",
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_next",
        },
        CoreMethod {
            name: "previous",
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_previous",
        },
        CoreMethod {
            name: "with",
            params: &[CoreTy::Options(WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_with",
        },
        CoreMethod {
            name: "startOf",
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_start_of",
        },
        CoreMethod {
            name: "endOf",
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_datetime_end_of",
        },
        CoreMethod {
            name: "difference",
            params: &[CoreTy::Instance(DATETIME_NAME), CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "mwl_core_time_datetime_difference",
        },
        CoreMethod {
            name: "toInstant",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_datetime_to_instant",
        },
        CoreMethod {
            name: "zone",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "mwl_core_time_datetime_zone",
        },
        CoreMethod {
            name: "weekday",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(WEEKDAY.name),
            symbol: "mwl_core_time_datetime_weekday",
        },
        CoreMethod {
            name: "dayOfYear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_time_datetime_day_of_year",
        },
        CoreMethod {
            name: "isLeapYear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_time_datetime_is_leap_year",
        },
    ],
    slots: &["seconds", "nanos", "zone"],
    constants: &[],
};

/// [`DATETIME`]'s slots, by index — the first two are [`INSTANT`]'s, in the
/// same order, so [`zoned_of`] can read either layout the same way.
const DATETIME_SECONDS_SLOT: usize = 0;
/// See [`DATETIME_SECONDS_SLOT`].
const DATETIME_NANOS_SLOT: usize = 1;
/// See [`DATETIME_SECONDS_SLOT`].
const DATETIME_ZONE_SLOT: usize = 2;

/// [`DATETIME`]'s symbols.
fn datetime_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_time_datetime_format" => (mwl_core_time_datetime_format as *const ()).cast(),
        "mwl_core_time_datetime_plus" => (mwl_core_time_datetime_plus as *const ()).cast(),
        "mwl_core_time_datetime_minus" => (mwl_core_time_datetime_minus as *const ()).cast(),
        "mwl_core_time_datetime_next" => (mwl_core_time_datetime_next as *const ()).cast(),
        "mwl_core_time_datetime_previous" => (mwl_core_time_datetime_previous as *const ()).cast(),
        "mwl_core_time_datetime_with" => (mwl_core_time_datetime_with as *const ()).cast(),
        "mwl_core_time_datetime_start_of" => (mwl_core_time_datetime_start_of as *const ()).cast(),
        "mwl_core_time_datetime_end_of" => (mwl_core_time_datetime_end_of as *const ()).cast(),
        "mwl_core_time_datetime_difference" => {
            (mwl_core_time_datetime_difference as *const ()).cast()
        }
        "mwl_core_time_datetime_to_instant" => {
            (mwl_core_time_datetime_to_instant as *const ()).cast()
        }
        "mwl_core_time_datetime_zone" => (mwl_core_time_datetime_zone as *const ()).cast(),
        "mwl_core_time_datetime_weekday" => (mwl_core_time_datetime_weekday as *const ()).cast(),
        "mwl_core_time_datetime_day_of_year" => {
            (mwl_core_time_datetime_day_of_year as *const ()).cast()
        }
        "mwl_core_time_datetime_is_leap_year" => {
            (mwl_core_time_datetime_is_leap_year as *const ()).cast()
        }
        _ => return None,
    })
}

// ============================================================================
// `Core\Time` — registration
// ============================================================================

/// `Core\Time`'s fully-qualified name, written once, for [`DURATION_NAME`]'s
/// reason.
pub const TIME_NAME: &str = r"Core\Time";

/// Spec § 4's entry points — the namespace class that produces the values the
/// three classes above are members of.
///
/// `parse` and `at` are deliberately absent: both answer with a `DateTime`,
/// which is gap 1 above.
pub const TIME: CoreClass = CoreClass {
    name: TIME_NAME,
    methods: &[
        CoreMethod {
            name: "now",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_now",
        },
        CoreMethod {
            name: "monotonic",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "mwl_core_time_monotonic",
        },
        CoreMethod {
            name: "sleep",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_time_sleep",
        },
        CoreMethod {
            name: "fromEpoch",
            params: &[
                CoreTy::Int,
                CoreTy::Options(&[CoreOption {
                    name: "nanos",
                    ty: CoreTy::Uint,
                    default: Const::Uint(0),
                }]),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_from_epoch",
        },
        CoreMethod {
            name: "fromIso",
            params: &[CoreTy::Str],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "mwl_core_time_from_iso",
        },
        CoreMethod {
            name: "parse",
            params: &[CoreTy::Str, CoreTy::Str, CoreTy::Instance(ZONE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_parse",
        },
        CoreMethod {
            name: "at",
            params: &[
                CoreTy::Int,
                CoreTy::Uint,
                CoreTy::Uint,
                CoreTy::Instance(ZONE_NAME),
                CoreTy::Options(&[
                    CoreOption {
                        name: "hour",
                        ty: CoreTy::Uint,
                        default: Const::Uint(0),
                    },
                    CoreOption {
                        name: "minute",
                        ty: CoreTy::Uint,
                        default: Const::Uint(0),
                    },
                    CoreOption {
                        name: "second",
                        ty: CoreTy::Uint,
                        default: Const::Uint(0),
                    },
                    CoreOption {
                        name: "nanos",
                        ty: CoreTy::Uint,
                        default: Const::Uint(0),
                    },
                ]),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "mwl_core_time_at",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// [`TIME`]'s symbols.
fn time_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_time_now" => (mwl_core_time_now as *const ()).cast(),
        "mwl_core_time_monotonic" => (mwl_core_time_monotonic as *const ()).cast(),
        "mwl_core_time_sleep" => (mwl_core_time_sleep as *const ()).cast(),
        "mwl_core_time_from_epoch" => (mwl_core_time_from_epoch as *const ()).cast(),
        "mwl_core_time_from_iso" => (mwl_core_time_from_iso as *const ()).cast(),
        "mwl_core_time_parse" => (mwl_core_time_parse as *const ()).cast(),
        "mwl_core_time_at" => (mwl_core_time_at as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Shared reading and building, for the three classes above
// ============================================================================

/// A fresh `Instant` at `at`.
fn instant_built(at: Timestamp) -> Value {
    crate::instance::build(
        &INSTANT,
        [
            Value::int(at.as_second()),
            Value::int(i64::from(at.subsec_nanosecond())),
        ],
    )
}

/// The `Instant` in argument slot `at` — slot 0 for a receiver, any other slot
/// for an `Instant` *parameter*.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives.
fn instant_of(args: &[Value], at: usize, member: &str) -> Result<Timestamp, Fault> {
    let object = crate::instance::receiver(args[at], &INSTANT, member)?;
    let seconds = crate::instance::slot(object, INSTANT_SECONDS_SLOT)
        .as_int()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Instant::{member} found a non-`int` `seconds` slot"
            ))
        })?;
    let nanos = crate::instance::slot(object, INSTANT_NANOS_SLOT)
        .as_int()
        .and_then(|held| i32::try_from(held).ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Instant::{member} found an out-of-range `nanos` slot"
            ))
        })?;
    Timestamp::new(seconds, nanos).map_err(|err| {
        Fault::fatal(format!(
            "Core\\Time\\Instant::{member} found an unrepresentable instant: {err}"
        ))
    })
}

/// ADR 0063 R4 for every class here but `Duration`, which has
/// [`overflowed`]'s one sentence: a result past what the type can hold throws
/// rather than wrapping. `member` is the whole `Core\…::name` label, since
/// four classes reach this.
fn out_of_range(member: &str, why: &str) -> Fault {
    Fault::thrown(format!("{member}(): {why}"))
}

/// The `string` in argument slot `at`, for a member that takes text.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives.
fn text_of<'a>(args: &'a [Value], at: usize, member: &str) -> Result<&'a str, Fault> {
    let bytes = args[at].as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string`, got tag {}",
            args[at].tag_byte()
        ))
    })?;
    // A `string` is guaranteed-valid UTF-8 (ADR 0009), so this cannot fail for
    // anything compiled code produced.
    std::str::from_utf8(bytes).map_err(|_| Fault::fatal(format!("{member} got invalid UTF-8")))
}

/// The exact number of nanoseconds from `earlier` to `later`, which is what
/// both `since` and `offsetAt` answer with a `Duration` built from.
///
/// Computed from the two slot fields rather than through a
/// [`SignedDuration`] so that the one range check is on the `i64` a `Duration`
/// actually holds — two timestamps a `Duration` cannot span throw here rather
/// than losing precision on the way.
fn nanos_between(later: Timestamp, earlier: Timestamp, member: &str) -> Result<i64, Fault> {
    let seconds = later.as_second().checked_sub(earlier.as_second());
    let subsec = i64::from(later.subsec_nanosecond()) - i64::from(earlier.subsec_nanosecond());
    seconds
        .and_then(|whole| whole.checked_mul(1_000_000_000))
        .and_then(|whole| whole.checked_add(subsec))
        .ok_or_else(|| {
            out_of_range(
                &format!(r"Core\Time\Instant::{member}"),
                "the two instants are further apart than a `Duration` can hold",
            )
        })
}

/// A fresh `Zone` with the id `id` — an IANA identifier, or a `±HH:MM[:SS]`
/// offset spelling for a fixed zone. See [`ZONE`].
fn zone_built(id: &str) -> Value {
    crate::instance::build(&ZONE, [Value::str(MwlStr::new(id.as_bytes()))])
}

/// The [`TimeZone`] the `Zone` in argument slot `at` names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a string, and a
/// [`Fault::thrown`] for an id the database no longer has — which only a
/// database swapped under a live process can produce, since
/// [`mwl_core_time_zone_of`] resolved it once already.
fn zone_of(args: &[Value], at: usize, member: &str) -> Result<TimeZone, Fault> {
    let object = crate::instance::receiver(args[at], &ZONE, member)?;
    let held = crate::instance::slot(object, ZONE_ID_SLOT);
    let bytes = held.as_str_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Time\\Zone::{member} found a non-`string` `id` slot"
        ))
    })?;
    // A `string` is guaranteed-valid UTF-8 (ADR 0009), and this crate wrote
    // this slot, so neither step can fail for anything a program can build.
    let id = std::str::from_utf8(bytes)
        .map_err(|_| Fault::fatal(format!("Core\\Time\\Zone::{member} found invalid UTF-8")))?;
    resolve_zone(id).ok_or_else(|| {
        Fault::thrown(format!(
            "Core\\Time\\Zone::{member}(): unknown time zone `{id}`"
        ))
    })
}

/// The [`TimeZone`] an id names, or `None` for one nothing does.
///
/// The whole of the two-spellings rule [`ZONE`] describes: no IANA identifier
/// begins with a sign, so the first byte decides which of the two an id is,
/// and nothing has to store a discriminant beside it.
fn resolve_zone(id: &str) -> Option<TimeZone> {
    if id.starts_with('+') || id.starts_with('-') {
        return parse_offset(id).map(TimeZone::fixed);
    }
    TimeZone::get(id).ok()
}

/// A `±HH:MM[:SS]` offset spelling, or `None` for anything else.
fn parse_offset(text: &str) -> Option<Offset> {
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let mut fields = text[1..].split(':');
    let hours: i32 = fields.next()?.parse().ok()?;
    let minutes: i32 = fields.next()?.parse().ok()?;
    let seconds: i32 = match fields.next() {
        Some(field) => field.parse().ok()?,
        None => 0,
    };
    if fields.next().is_some() || !(0..60).contains(&minutes) || !(0..60).contains(&seconds) {
        return None;
    }
    let total = hours.checked_mul(3_600)? + minutes * 60 + seconds;
    Offset::from_seconds(sign * total).ok()
}

/// The `±HH:MM[:SS]` spelling of a whole-second offset — [`parse_offset`]'s
/// inverse, and the id a fixed [`ZONE`] is stored under.
fn render_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let total = seconds.unsigned_abs();
    let (hours, minutes, seconds) = (total / 3_600, (total % 3_600) / 60, total % 60);
    if seconds == 0 {
        format!("{sign}{hours:02}:{minutes:02}")
    } else {
        format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
    }
}

// ============================================================================
// `Core\Time`'s entry points
// ============================================================================

/// This process's monotonic origin, fixed at the first
/// [`mwl_core_time_monotonic`] call.
///
/// Process-wide rather than per-core so that two cores' readings are
/// differences on one timeline, which is the only thing a monotonic clock is
/// for; written once and read thereafter, so the sharing costs no
/// synchronisation past the first call.
static MONOTONIC_ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();

mwl_runtime::mwl_helper! {
    /// `Core\Time::now(): Instant` — the wall clock, which replaces `time`,
    /// `microtime` and `date_create` at once.
    fn mwl_core_time_now(_ctx, args: [0]) {
        let _ = args;
        Ok(instant_built(Timestamp::now()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::monotonic(): Duration` — for measuring, never for
    /// wall-clock, which is why it answers with a `Duration` and not an
    /// `Instant`: the value has no meaning except against another reading of
    /// the same clock.
    fn mwl_core_time_monotonic(_ctx, args: [0]) {
        let _ = args;
        let origin = MONOTONIC_ORIGIN.get_or_init(std::time::Instant::now);
        Ok(built(
            i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX),
        ))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::sleep(Duration $d): void`, replacing `sleep`, `usleep`,
    /// `time_nanosleep` and `time_sleep_until` — one member, because the four
    /// differ only in the unit a `Duration` already carries.
    ///
    /// A negative or zero duration returns at once rather than throwing:
    /// "sleep until a moment already past" is a wait of no time, which is what
    /// every caller computing a deadline wants. **Blocks the core thread** —
    /// this module's gap 3 owns why, and the signature does not change when
    /// M5's scheduler makes it a suspension point.
    fn mwl_core_time_sleep(_ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "sleep")?;
        if nanos > 0 {
            std::thread::sleep(std::time::Duration::from_nanos(nanos.unsigned_abs()));
        }
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::fromEpoch(int $seconds, {nanos?: uint}): Instant`.
    ///
    /// The `nanos` option is added *after* the second, rather than written
    /// into the slot beside it, so that a negative second and a positive
    /// `nanos` mean what a reader expects — one nanosecond after
    /// `-1`, not one before it.
    fn mwl_core_time_from_epoch(_ctx, args: [2]) {
        let seconds = count(args, 0, "fromEpoch")?;
        let nanos = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time::fromEpoch expected a `uint` for `nanos`, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        if nanos >= 1_000_000_000 {
            return Err(out_of_range(
                r"Core\Time::fromEpoch",
                "`nanos` is a subsecond count, so it is below 1000000000",
            ));
        }
        let at = Timestamp::from_second(seconds)
            .map_err(|err| out_of_range(r"Core\Time::fromEpoch", &err.to_string()))?;
        #[expect(
            clippy::cast_possible_wrap,
            reason = "the bound checked one line above puts `nanos` below \
                      1e9, which is well inside `i64`"
        )]
        let offset = SignedDuration::from_nanos(nanos as i64);
        at.checked_add(offset)
            .map(instant_built)
            .map_err(|err| out_of_range(r"Core\Time::fromEpoch", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::fromIso(string $text): Instant` — an ISO-8601 timestamp
    /// that carries its own offset, which is the whole of what `strtotime`
    /// replaces here. A relative expression is not accepted in either half
    /// (§ 4), and a *civil* time with no offset is `Time::parse`, which needs
    /// a zone.
    fn mwl_core_time_from_iso(_ctx, args: [1]) {
        let text = text_of(args, 0, "Core\\Time::fromIso")?;
        text.parse::<Timestamp>()
            .map(instant_built)
            .map_err(|err| Fault::thrown(format!("Core\\Time::fromIso(): {err}")))
    }
}

// ============================================================================
// `Core\Time\Instant`'s members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `$i->toEpochSeconds(): int` — replaces `getTimestamp` and `date("U")`.
    fn mwl_core_time_instant_to_epoch_seconds(_ctx, args: [1]) {
        Ok(Value::int(instant_of(args, 0, "toEpochSeconds")?.as_second()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->toEpochMillis(): int` — one of `microtime(true)`'s two halves,
    /// as an exact integer rather than a `float` that loses the subsecond
    /// digits it was asked for.
    fn mwl_core_time_instant_to_epoch_millis(_ctx, args: [1]) {
        let at = instant_of(args, 0, "toEpochMillis")?;
        at.as_second()
            .checked_mul(1_000)
            .and_then(|whole| {
                whole.checked_add(i64::from(at.subsec_nanosecond()) / 1_000_000)
            })
            .map(Value::int)
            .ok_or_else(|| {
                out_of_range(
                    r"Core\Time\Instant::toEpochMillis",
                    "this instant is past what a millisecond count can hold",
                )
            })
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->toEpochMicros(): int` — `microtime`'s other half.
    fn mwl_core_time_instant_to_epoch_micros(_ctx, args: [1]) {
        let at = instant_of(args, 0, "toEpochMicros")?;
        at.as_second()
            .checked_mul(1_000_000)
            .and_then(|whole| whole.checked_add(i64::from(at.subsec_nanosecond()) / 1_000))
            .map(Value::int)
            .ok_or_else(|| {
                out_of_range(
                    r"Core\Time\Instant::toEpochMicros",
                    "this instant is past what a microsecond count can hold",
                )
            })
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->plus(Duration $d): Instant` — the exact arithmetic of § 4's two,
    /// so it crosses a DST boundary without noticing one. A calendar step is
    /// `DateTime::plus($n, Unit::Day)`.
    fn mwl_core_time_instant_plus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "plus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "plus")?);
        at.checked_add(by)
            .map(instant_built)
            .map_err(|err| out_of_range(r"Core\Time\Instant::plus", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->minus(Duration $d): Instant`.
    fn mwl_core_time_instant_minus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "minus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "minus")?);
        at.checked_sub(by)
            .map(instant_built)
            .map_err(|err| out_of_range(r"Core\Time\Instant::minus", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->since(Instant $earlier): Duration` — replaces `date_diff` and
    /// `DateInterval` arithmetic, with none of that type's "1 month" ambiguity
    /// because the answer is an exact count.
    ///
    /// Negative where `$earlier` is in fact later, which is what makes it the
    /// inverse of `plus` rather than an absolute distance.
    fn mwl_core_time_instant_since(_ctx, args: [2]) {
        let later = instant_of(args, 0, "since")?;
        let earlier = instant_of(args, 1, "since")?;
        Ok(built(nanos_between(later, earlier, "since")?))
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->compareTo(Instant $other): int` — `Comparable`'s member
    /// ([ADR 0013](../../../../docs/adr/0013-comparable-interface.md)).
    fn mwl_core_time_instant_compare_to(_ctx, args: [2]) {
        let left = instant_of(args, 0, "compareTo")?;
        let right = instant_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `$i->toIso(): string` — RFC 3339 in UTC, which is `date(DATE_ATOM)`'s
    /// replacement and the one rendering that needs no zone argument.
    fn mwl_core_time_instant_to_iso(_ctx, args: [1]) {
        let at = instant_of(args, 0, "toIso")?;
        Ok(Value::str(MwlStr::new(at.to_string().as_bytes())))
    }
}

// ============================================================================
// `Core\Time\Zone`'s members
// ============================================================================

mwl_runtime::mwl_helper! {
    /// `Core\Time\Zone::of(string $id): Zone` — an IANA identifier, replacing
    /// `DateTimeZone`. Throws on an unknown one rather than falling back to
    /// UTC, which is PHP's behaviour and the reason a mistyped zone there is a
    /// silent wrong answer.
    fn mwl_core_time_zone_of(_ctx, args: [1]) {
        let id = text_of(args, 0, "Core\\Time\\Zone::of")?;
        // A sign-leading id is `fixed`'s spelling, not an IANA one, so `of`
        // refuses it: two ways to build one value is exactly what ADR 0063
        // R20 forbids.
        if TimeZone::get(id).is_err() || id.starts_with('+') || id.starts_with('-') {
            return Err(Fault::thrown(format!(
                "Core\\Time\\Zone::of(): unknown time zone `{id}`"
            )));
        }
        Ok(zone_built(id))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Zone::fixed(Duration $offset): Zone` — for a timestamp that
    /// carries an offset instead of a region, which is every RFC 3339 string
    /// and no IANA identifier.
    ///
    /// A whole number of seconds, within the ±25:59:59 the IANA format itself
    /// allows; anything else throws rather than rounding.
    fn mwl_core_time_zone_fixed(_ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "fixed")?;
        if nanos % 1_000_000_000 != 0 {
            return Err(out_of_range(
                r"Core\Time\Zone::fixed",
                "a zone offset is a whole number of seconds",
            ));
        }
        let seconds = i32::try_from(nanos / 1_000_000_000).ok();
        let offset = seconds.and_then(|seconds| Offset::from_seconds(seconds).ok());
        match offset {
            Some(offset) => Ok(zone_built(&render_offset(offset.seconds()))),
            None => Err(out_of_range(
                r"Core\Time\Zone::fixed",
                "a zone offset is within 25:59:59 of UTC",
            )),
        }
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time\Zone::system(): Zone` — the host's configured zone,
    /// replacing `date_default_timezone_get`.
    ///
    /// **Not** an ambient default (§ 4): it is an ordinary value a program
    /// asks for and then passes explicitly, so a call site still names the
    /// zone it converts in. There is no `date_default_timezone_set`.
    fn mwl_core_time_zone_system(_ctx, args: [0]) {
        let _ = args;
        let zone = TimeZone::system();
        Ok(match zone.iana_name() {
            Some(name) => zone_built(name),
            // A host with no IANA name — a bare `TZ=+02:00`, or a Windows zone
            // with no mapping — still has an offset, so it becomes the fixed
            // zone it in fact is rather than an error at startup.
            None => zone_built(&render_offset(zone.to_offset(Timestamp::now()).seconds())),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `$z->offsetAt(Instant $i): Duration` — replaces `getOffset`.
    ///
    /// Takes the instant because an offset is not a property of a zone:
    /// `Europe/Berlin` is `+01:00` in January and `+02:00` in July, and a
    /// member that did not ask would have to pick one silently.
    fn mwl_core_time_zone_offset_at(_ctx, args: [2]) {
        let zone = zone_of(args, 0, "offsetAt")?;
        let at = instant_of(args, 1, "offsetAt")?;
        Ok(built(i64::from(zone.to_offset(at).seconds()) * 1_000_000_000))
    }
}

// ============================================================================
// `Core\Time\DateTime` — reading, building and the calendar step
// ============================================================================

/// A fresh `DateTime` at `at`, in `at`'s own zone.
///
/// The zone id is re-derived from the [`TimeZone`] rather than carried through
/// from the caller so that every path stores the same spelling: an IANA name
/// where there is one, and [`render_offset`]'s `±HH:MM[:SS]` where there is
/// not, which is exactly [`ZONE`]'s rule.
fn datetime_built(at: &Zoned) -> Value {
    let id = match at.time_zone().iana_name() {
        Some(name) => name.to_owned(),
        None => render_offset(at.offset().seconds()),
    };
    crate::instance::build(
        &DATETIME,
        [
            Value::int(at.timestamp().as_second()),
            Value::int(i64::from(at.timestamp().subsec_nanosecond())),
            Value::str(MwlStr::new(id.as_bytes())),
        ],
    )
}

/// The [`Zoned`] the `DateTime` in argument slot `at` holds.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives, and a
/// [`Fault::thrown`] for a zone id the database no longer has — the same
/// unreachable-in-practice case [`zone_of`] describes.
fn zoned_of(args: &[Value], at: usize, member: &str) -> Result<Zoned, Fault> {
    let object = crate::instance::receiver(args[at], &DATETIME, member)?;
    let read = |index: usize, what: &str| {
        crate::instance::slot(object, index)
            .as_int()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Time\\DateTime::{member} found a non-`int` `{what}` slot"
                ))
            })
    };
    let seconds = read(DATETIME_SECONDS_SLOT, "seconds")?;
    let nanos = i32::try_from(read(DATETIME_NANOS_SLOT, "nanos")?).map_err(|_| {
        Fault::fatal(format!(
            "Core\\Time\\DateTime::{member} found an out-of-range `nanos` slot"
        ))
    })?;
    let held = crate::instance::slot(object, DATETIME_ZONE_SLOT);
    let id = held
        .as_str_bytes()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\DateTime::{member} found a non-`string` `zone` slot"
            ))
        })?;
    let zone = resolve_zone(id).ok_or_else(|| {
        Fault::thrown(format!(
            "Core\\Time\\DateTime::{member}(): unknown time zone `{id}`"
        ))
    })?;
    let timestamp = Timestamp::new(seconds, nanos).map_err(|err| {
        Fault::fatal(format!(
            "Core\\Time\\DateTime::{member} found an unrepresentable instant: {err}"
        ))
    })?;
    Ok(Zoned::new(timestamp, zone))
}

/// The [`jiff::Unit`] a `Core\Unit` case names, and how many of it one case
/// step is — the pair exists for `Quarter`, which [`jiff`] does not carry and
/// which is three months by definition rather than by convention.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives: the
/// checker has already refused anything that is not a case of this enum.
fn unit_of(args: &[Value], at: usize, member: &str) -> Result<(jiff::Unit, i64), Fault> {
    match args[at].as_int() {
        Some(0) => Ok((jiff::Unit::Nanosecond, 1)),
        Some(1) => Ok((jiff::Unit::Microsecond, 1)),
        Some(2) => Ok((jiff::Unit::Millisecond, 1)),
        Some(3) => Ok((jiff::Unit::Second, 1)),
        Some(4) => Ok((jiff::Unit::Minute, 1)),
        Some(5) => Ok((jiff::Unit::Hour, 1)),
        Some(6) => Ok((jiff::Unit::Day, 1)),
        Some(7) => Ok((jiff::Unit::Week, 1)),
        Some(8) => Ok((jiff::Unit::Month, 1)),
        Some(9) => Ok((jiff::Unit::Month, 3)),
        Some(10) => Ok((jiff::Unit::Year, 1)),
        other => Err(Fault::fatal(format!(
            "{member} expected a `Core\\Unit` case, got {other:?}"
        ))),
    }
}

/// The [`Weekday`] a `Core\Weekday` case names.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`unit_of`]'s reason.
fn weekday_of(args: &[Value], at: usize, member: &str) -> Result<Weekday, Fault> {
    args[at]
        .as_int()
        .and_then(|held| i8::try_from(held).ok())
        .and_then(|index| Weekday::from_monday_zero_offset(index).ok())
        .ok_or_else(|| Fault::fatal(format!("{member} expected a `Core\\Weekday` case")))
}

/// `value` of `unit` as a [`jiff::Span`].
///
/// One arm per unit rather than `Span`'s own private `try_unit`, which is the
/// only thing in this module that reaches for a `jiff` internal — a public
/// setter per unit is what that crate exposes, so the dispatch is ours.
fn span_of(unit: jiff::Unit, value: i64) -> Result<jiff::Span, jiff::Error> {
    let span = jiff::Span::new();
    match unit {
        jiff::Unit::Year => span.try_years(value),
        jiff::Unit::Month => span.try_months(value),
        jiff::Unit::Week => span.try_weeks(value),
        jiff::Unit::Day => span.try_days(value),
        jiff::Unit::Hour => span.try_hours(value),
        jiff::Unit::Minute => span.try_minutes(value),
        jiff::Unit::Second => span.try_seconds(value),
        jiff::Unit::Millisecond => span.try_milliseconds(value),
        jiff::Unit::Microsecond => span.try_microseconds(value),
        jiff::Unit::Nanosecond => span.try_nanoseconds(value),
    }
}

/// `count` steps of the unit in argument slot 2, added to the receiver.
///
/// Both `plus` and `minus` reach here, `minus` with the count negated, so the
/// two cannot come to mean different things at a month end or a DST boundary.
fn stepped(args: &[Value], member: &str, sign: i64) -> Result<Value, Fault> {
    let at = zoned_of(args, 0, member)?;
    let (unit, scale) = unit_of(args, 2, member)?;
    let steps = count(args, 1, member)?
        .checked_mul(scale)
        .and_then(|steps| steps.checked_mul(sign))
        .ok_or_else(|| {
            out_of_range(
                &format!(r"Core\Time\DateTime::{member}"),
                "that many units is past what a calendar span can hold",
            )
        })?;
    let span = span_of(unit, steps)
        .map_err(|err| out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string()))?;
    at.checked_add(span)
        .map(|moved| datetime_built(&moved))
        .map_err(|err| out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string()))
}

/// The receiver truncated down to the start of `unit`.
///
/// Computed on the **civil** fields and then placed back in the zone, which is
/// what makes `startOf(Unit::Day)` the local midnight rather than a fixed
/// number of hours back: § 4's calendar arithmetic is civil arithmetic, and
/// the zone decides afterwards what instant that names.
fn floored(at: &Zoned, unit: jiff::Unit, scale: i64, member: &str) -> Result<Zoned, Fault> {
    let civil = at.datetime();
    let (date, time) = (civil.date(), civil.time());
    let floored = match unit {
        jiff::Unit::Year => civil::DateTime::from_parts(
            civil::Date::new(date.year(), 1, 1).map_err(|err| {
                out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string())
            })?,
            civil::Time::midnight(),
        ),
        // One `Quarter` is three months, so its start is the first month of
        // the three the receiver's own month falls in.
        jiff::Unit::Month if scale == 3 => {
            let month = (date.month() - 1) / 3 * 3 + 1;
            civil::DateTime::from_parts(
                civil::Date::new(date.year(), month, 1).map_err(|err| {
                    out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string())
                })?,
                civil::Time::midnight(),
            )
        }
        jiff::Unit::Month => {
            civil::DateTime::from_parts(date.first_of_month(), civil::Time::midnight())
        }
        // ISO-8601's week, which starts on Monday — the same order
        // `Core\Weekday`'s cases are in.
        jiff::Unit::Week => civil::DateTime::from_parts(
            date.checked_sub(
                jiff::Span::new().days(i64::from(date.weekday().to_monday_zero_offset())),
            )
            .map_err(|err| {
                out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string())
            })?,
            civil::Time::midnight(),
        ),
        jiff::Unit::Day => civil::DateTime::from_parts(date, civil::Time::midnight()),
        jiff::Unit::Hour => civil::DateTime::from_parts(
            date,
            civil::Time::new(time.hour(), 0, 0, 0).unwrap_or(civil::Time::midnight()),
        ),
        jiff::Unit::Minute => civil::DateTime::from_parts(
            date,
            civil::Time::new(time.hour(), time.minute(), 0, 0).unwrap_or(civil::Time::midnight()),
        ),
        jiff::Unit::Second => civil::DateTime::from_parts(
            date,
            civil::Time::new(time.hour(), time.minute(), time.second(), 0)
                .unwrap_or(civil::Time::midnight()),
        ),
        // The three subsecond units differ only in how many of the nine digits
        // survive, so they share one arm.
        smaller => {
            let divisor = match smaller {
                jiff::Unit::Millisecond => 1_000_000,
                jiff::Unit::Microsecond => 1_000,
                _ => 1,
            };
            civil::DateTime::from_parts(
                date,
                civil::Time::new(
                    time.hour(),
                    time.minute(),
                    time.second(),
                    time.subsec_nanosecond() / divisor * divisor,
                )
                .unwrap_or(civil::Time::midnight()),
            )
        }
    };
    at.time_zone()
        .to_zoned(floored)
        .map_err(|err| out_of_range(&format!(r"Core\Time\DateTime::{member}"), &err.to_string()))
}

mwl_runtime::mwl_helper! {
    /// `$d->format(string $pattern): string` — CLDR patterns
    /// ([`crate::cldr`]), replacing `date`, `gmdate`, `idate`, `strftime` and
    /// `date_format` at once.
    ///
    /// The pattern is compiled per call; ADR 0057 is what moves that to
    /// compile time for a literal one, and [`crate::cldr`]'s gap 1 owns it.
    fn mwl_core_time_datetime_format(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "format")?;
        let pattern = text_of(args, 1, "Core\\Time\\DateTime::format")?;
        let pieces = crate::cldr::compile(pattern)
            .map_err(|why| Fault::thrown(format!("Core\\Time\\DateTime::format(): {why}")))?;
        Ok(Value::str(MwlStr::new(
            crate::cldr::render(&pieces, &at).as_bytes(),
        )))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->plus(int $count, Unit $unit): DateTime` — § 4's *calendar* half,
    /// so adding `1, Unit::Month` lands on the same day-of-month clamped to
    /// the month's length, and crossing a DST boundary makes a 23- or 25-hour
    /// day. The exact half is `$d->toInstant()->plus(24h)`.
    fn mwl_core_time_datetime_plus(_ctx, args: [3]) {
        stepped(args, "plus", 1)
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->minus(int $count, Unit $unit): DateTime`.
    fn mwl_core_time_datetime_minus(_ctx, args: [3]) {
        stepped(args, "minus", -1)
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->next(Weekday $w): DateTime` — the nearest **strictly later** day
    /// with that weekday, time-of-day preserved. Replaces
    /// `strtotime("next monday")`.
    fn mwl_core_time_datetime_next(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "next")?;
        let weekday = weekday_of(args, 1, "Core\\Time\\DateTime::next")?;
        at.nth_weekday(1, weekday)
            .map(|found| datetime_built(&found))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::next", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->previous(Weekday $w): DateTime` — [`mwl_core_time_datetime_next`]
    /// backwards, and strictly earlier for the same reason.
    fn mwl_core_time_datetime_previous(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "previous")?;
        let weekday = weekday_of(args, 1, "Core\\Time\\DateTime::previous")?;
        at.nth_weekday(-1, weekday)
            .map(|found| datetime_built(&found))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::previous", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->with({year?, month?, day?, hour?, minute?, second?, nanos?}):
    /// DateTime` — replaces `setDate`, `setTime` and `setISODate`.
    ///
    /// An omitted option arrives as `Tag::Null` and leaves its field alone;
    /// see [`WITH_OPTIONS`] for why that is a tag rather than a sentinel.
    fn mwl_core_time_datetime_with(_ctx, args: [8]) {
        let at = zoned_of(args, 0, "with")?;
        let mut building = at.with();
        if let Some(year) = optional(args, 1, "with", "year")? {
            building = building.year(
                i16::try_from(year).map_err(|_| {
                    out_of_range(r"Core\Time\DateTime::with", "`year` is outside -9999..=9999")
                })?,
            );
        }
        for (index, name) in [(2, "month"), (3, "day"), (4, "hour"), (5, "minute"), (6, "second")] {
            let Some(value) = optional(args, index, "with", name)? else {
                continue;
            };
            let value = i8::try_from(value).map_err(|_| {
                out_of_range(
                    r"Core\Time\DateTime::with",
                    &format!("`{name}` is outside what that field can hold"),
                )
            })?;
            building = match name {
                "month" => building.month(value),
                "day" => building.day(value),
                "hour" => building.hour(value),
                "minute" => building.minute(value),
                _ => building.second(value),
            };
        }
        if let Some(nanos) = optional(args, 7, "with", "nanos")? {
            building = building.subsec_nanosecond(
                i32::try_from(nanos).map_err(|_| {
                    out_of_range(r"Core\Time\DateTime::with", "`nanos` is a subsecond count")
                })?,
            );
        }
        building
            .build()
            .map(|built| datetime_built(&built))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::with", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->startOf(Unit $u): DateTime`.
    fn mwl_core_time_datetime_start_of(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "startOf")?;
        let (unit, scale) = unit_of(args, 1, "Core\\Time\\DateTime::startOf")?;
        Ok(datetime_built(&floored(&at, unit, scale, "startOf")?))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->endOf(Unit $u): DateTime` — the **last** instant of the unit,
    /// which is one nanosecond before the next one starts.
    ///
    /// A distinct operation from `startOf` at a DST boundary, which is § 4's
    /// stated reason both exist: the two are not a fixed distance apart on a
    /// day the zone lengthens or shortens.
    fn mwl_core_time_datetime_end_of(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "endOf")?;
        let (unit, scale) = unit_of(args, 1, "Core\\Time\\DateTime::endOf")?;
        let start = floored(&at, unit, scale, "endOf")?;
        let step = span_of(unit, scale)
            .map_err(|err| out_of_range(r"Core\Time\DateTime::endOf", &err.to_string()))?;
        start
            .checked_add(step)
            .and_then(|next| next.checked_sub(jiff::Span::new().nanoseconds(1)))
            .map(|found| datetime_built(&found))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::endOf", &err.to_string()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->difference(DateTime $other, Unit $unit): int` — whole units from
    /// the receiver **to** `$other`, so an age is
    /// `$birth->difference($today, Unit::Year)` and a past date answers
    /// negative. Replaces `date_diff` plus `DateInterval`'s `y`/`m`/`d`.
    ///
    /// **Counted in the receiver's zone.** A calendar unit is only defined
    /// against one calendar, so `$other` is moved into the receiver's zone
    /// first — the same instant, seen from where the question was asked. The
    /// alternative, refusing two different zones, would make the common case
    /// (a UTC timestamp against a user's local day) a throw rather than an
    /// answer, and there is no third zone either operand could name.
    fn mwl_core_time_datetime_difference(_ctx, args: [3]) {
        let from = zoned_of(args, 0, "difference")?;
        let to = zoned_of(args, 1, "difference")?.with_time_zone(from.time_zone().clone());
        let (unit, scale) = unit_of(args, 2, "Core\\Time\\DateTime::difference")?;
        let span = from
            .until(jiff::ZonedDifference::new(&to).largest(unit))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::difference", &err.to_string()))?;
        let whole = match unit {
            jiff::Unit::Year => i64::from(span.get_years()),
            jiff::Unit::Month => i64::from(span.get_months()),
            jiff::Unit::Week => i64::from(span.get_weeks()),
            jiff::Unit::Day => i64::from(span.get_days()),
            jiff::Unit::Hour => i64::from(span.get_hours()),
            jiff::Unit::Minute => span.get_minutes(),
            jiff::Unit::Second => span.get_seconds(),
            jiff::Unit::Millisecond => span.get_milliseconds(),
            jiff::Unit::Microsecond => span.get_microseconds(),
            _ => span.get_nanoseconds(),
        };
        Ok(Value::int(whole / scale))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->toInstant(): Instant` — free, since a `DateTime` already holds
    /// one ([`DATETIME`]).
    fn mwl_core_time_datetime_to_instant(_ctx, args: [1]) {
        Ok(instant_built(zoned_of(args, 0, "toInstant")?.timestamp()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->zone(): Zone` — the zone this civil time is in, which is never
    /// ambient and so is always one the program named.
    fn mwl_core_time_datetime_zone(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "zone")?;
        Ok(match at.time_zone().iana_name() {
            Some(name) => zone_built(name),
            None => zone_built(&render_offset(at.offset().seconds())),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->weekday(): Weekday` — replaces `date("N")`, as the enum case
    /// rather than the number.
    fn mwl_core_time_datetime_weekday(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "weekday")?;
        Ok(Value::int(i64::from(
            at.weekday().to_monday_zero_offset(),
        )))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->dayOfYear(): uint` — replaces `date("z")`, one-based where PHP's
    /// is zero-based, because every other day count in § 4 is one-based too.
    fn mwl_core_time_datetime_day_of_year(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "dayOfYear")?;
        Ok(Value::uint(at.day_of_year().unsigned_abs().into()))
    }
}

mwl_runtime::mwl_helper! {
    /// `$d->isLeapYear(): bool` — replaces `date("L")` and `checkdate`'s year
    /// half; the day half has no equivalent because an invalid date throws
    /// where it is built.
    fn mwl_core_time_datetime_is_leap_year(_ctx, args: [1]) {
        Ok(Value::bool(zoned_of(args, 0, "isLeapYear")?.in_leap_year()))
    }
}

/// The `uint`-or-`null` in argument slot `at` — an ADR 0063 R2 option whose
/// omitted spelling is [`Const::Null`], for a member that has no in-range
/// sentinel to use instead.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member and the option, for the reason
/// [`count`] gives.
fn optional(args: &[Value], at: usize, member: &str, option: &str) -> Result<Option<i64>, Fault> {
    if args[at].tag() == Some(mwl_runtime::Tag::Null) {
        return Ok(None);
    }
    args[at]
        .as_int()
        .or_else(|| args[at].as_uint().and_then(|held| i64::try_from(held).ok()))
        .map(Some)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\DateTime::{member} expected a number for `{option}`, got tag {}",
                args[at].tag_byte()
            ))
        })
}

mwl_runtime::mwl_helper! {
    /// `$i->in(Zone $zone): DateTime` — the only instant→calendar conversion
    /// there is, and the reason § 4 needs no ambient timezone.
    fn mwl_core_time_instant_in(_ctx, args: [2]) {
        let at = instant_of(args, 0, "in")?;
        let zone = zone_of(args, 1, "in")?;
        Ok(datetime_built(&Zoned::new(at, zone)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::parse(string $text, string $format, Zone $zone): DateTime`
    /// — `DateTime::createFromFormat` and `strptime`, over [`crate::cldr`]'s
    /// patterns rather than PHP's letters.
    ///
    /// The zone is the third argument rather than something the pattern can
    /// name, which is why a zonal field in the pattern is refused: two answers
    /// for one question is what ADR 0063 R20 leaves no room for.
    fn mwl_core_time_parse(_ctx, args: [3]) {
        let text = text_of(args, 0, "Core\\Time::parse")?;
        let pattern = text_of(args, 1, "Core\\Time::parse")?;
        let zone = zone_of(args, 2, "parse")?;
        let pieces = crate::cldr::compile(pattern)
            .map_err(|why| Fault::thrown(format!("Core\\Time::parse(): {why}")))?;
        crate::cldr::read(&pieces, text, &zone)
            .map(|at| datetime_built(&at))
            .map_err(|why| Fault::thrown(format!("Core\\Time::parse(): {why}")))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Time::at(int $year, uint $month, uint $day, Zone $zone,
    /// {hour?, minute?, second?, nanos?}): DateTime` — replaces `mktime`,
    /// `gmmktime` and `DateTime::setDate`.
    ///
    /// A civil time the zone skips over — 02:30 on a spring-forward morning —
    /// resolves forward rather than throwing, which is [`jiff`]'s
    /// *compatible* disambiguation and the rule every other language's
    /// calendar library settled on. There is no `checkdate`: a date that does
    /// not exist at all throws here.
    fn mwl_core_time_at(_ctx, args: [8]) {
        let year = i16::try_from(count(args, 0, "at")?).map_err(|_| {
            out_of_range(r"Core\Time::at", "a year is within -9999..=9999")
        })?;
        let mut fields = [0_i8; 5];
        for (slot, (index, name)) in fields.iter_mut().zip([
            (1, "month"),
            (2, "day"),
            (4, "hour"),
            (5, "minute"),
            (6, "second"),
        ]) {
            let held = args[index].as_uint().ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Time::at expected a `uint` for `{name}`, got tag {}",
                    args[index].tag_byte()
                ))
            })?;
            *slot = i8::try_from(held).map_err(|_| {
                out_of_range(r"Core\Time::at", &format!("`{name}` is out of range"))
            })?;
        }
        let nanos = args[7]
            .as_uint()
            .and_then(|held| i32::try_from(held).ok())
            .ok_or_else(|| out_of_range(r"Core\Time::at", "`nanos` is a subsecond count"))?;
        let zone = zone_of(args, 3, "at")?;
        let civil = civil::DateTime::new(
            year, fields[0], fields[1], fields[2], fields[3], fields[4], nanos,
        )
        .map_err(|err| out_of_range(r"Core\Time::at", &err.to_string()))?;
        zone.to_zoned(civil)
            .map(|at| datetime_built(&at))
            .map_err(|err| out_of_range(r"Core\Time::at", &err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every slot constant and the registered layout agree — the one thing a
    /// paste error here would turn into a silent wrong read.
    #[test]
    fn the_slot_constants_match_the_registered_layouts() {
        assert_eq!(NANOS_SLOT, DURATION.slot("nanos"));
        assert_eq!(DURATION.slots.len(), 1);
        assert_eq!(INSTANT_SECONDS_SLOT, INSTANT.slot("seconds"));
        assert_eq!(INSTANT_NANOS_SLOT, INSTANT.slot("nanos"));
        assert_eq!(INSTANT.slots.len(), 2);
        assert_eq!(ZONE_ID_SLOT, ZONE.slot("id"));
        assert_eq!(ZONE.slots.len(), 1);
    }

    /// Every symbol this module's four classes register resolves in its own
    /// `address` — the miss that is a *runtime* panic rather than a link
    /// error, so it is worth a test of its own.
    #[test]
    fn every_registered_symbol_has_an_address_here() {
        for class in [&TIME, &INSTANT, &DURATION, &ZONE] {
            for method in class.members() {
                assert!(
                    address(method.symbol).is_some(),
                    "{} has no address",
                    method.symbol
                );
            }
        }
    }

    /// A fixed zone's id round-trips through the `±HH:MM[:SS]` spelling
    /// [`ZONE`] stores it under — the property that lets one `string` slot
    /// carry both kinds of zone, so it is checked rather than assumed.
    #[test]
    fn a_fixed_offset_round_trips_through_its_id() {
        for seconds in [0, 3_600, -3_600, 19_800, -19_800, 45, 93_599, -93_599] {
            let id = render_offset(seconds);
            let parsed = parse_offset(&id).unwrap_or_else(|| panic!("`{id}` does not parse"));
            assert_eq!(parsed.seconds(), seconds, "`{id}` came back wrong");
            assert!(
                resolve_zone(&id).is_some(),
                "`{id}` does not resolve to a zone"
            );
        }
    }

    /// No IANA identifier begins with a sign, which is the whole of why one
    /// slot can hold both spellings — so the two are checked not to overlap.
    #[test]
    fn an_offset_spelling_is_never_mistaken_for_an_iana_name() {
        assert!(parse_offset("Europe/Berlin").is_none());
        assert!(parse_offset("UTC").is_none());
        assert!(parse_offset("+02").is_none());
        assert!(parse_offset("+02:60").is_none());
        assert!(parse_offset("+99:00").is_none());
        assert!(resolve_zone("+02:00").is_some());
        assert!(resolve_zone("Nowhere/Nothing").is_none());
    }
}
