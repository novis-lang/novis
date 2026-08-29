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
//! [`nvs_syntax::duration`], which the lexer calls for the source literal and
//! `nvs.toml` will call for a duration-valued directive. ADR 0070 § 5 requires
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
//! all, which is the same thing `nvs_runtime`'s own gap 3 owes a string
//! literal, so both close together rather than one growing a mechanism the
//! other does not use. Until then `30s` is one
//! [`nvs_core_time_duration_nanoseconds`] call on a folded constant, which is
//! already the whole grammar resolved at compile time.
//!
//! # Known gaps
//!
//! 1. **There is no `Core\Month`, and there is not going to be one.** § 4
//!    writes no member that takes or answers with one, `Core\Weekday` existing
//!    only because `$d->weekday()` does, and an enum nothing names is surface
//!    with no spec home. This is item 1 because it is the question that keeps
//!    being re-asked, not because anything is missing: three earlier notes
//!    here described a member the spec never had. **§ 4 itself is whole** —
//!    `withTime`, its last row and its only one that *takes* a component view
//!    rather than answering with one, is
//!    [`nvs_core_time_datetime_with_time`].
//! 2. **`$d->format` and `Core\Time::parse` compile their pattern per call.**
//!    ADR 0057 makes both intrinsics whose literal pattern is prepared while
//!    compiling; [`crate::cldr`]'s own gap 1 owns what that changes and what
//!    it does not.
//! 3. **`Core\Time::sleep` parks the task rather than blocking the core**, and
//!    what is left of it is one thing: a wait *off* a core still blocks the
//!    thread it is on, which is right for `nvs run` and is what a worker's
//!    blocking pool would otherwise be for. The route is
//!    [`nvs_runtime::host::Host::sleep`] and the mechanism is `nvs-host`'s
//!    `timer` module — this task's own deadline on the reactor the core is
//!    about to poll. A cancellation mid-sleep comes back as
//!    [`nvs_runtime::host::Woken::Cancelled`] and the member stops the request
//!    through [`nvs_runtime::Ctx::cancel`], because a parked task standing on
//!    an `extern "C"` frame is resumed rather than unwound; `nvs_host`'s
//!    scheduler module doc owns that decision.
//! 4. **`Comparable` and `Stringable` are satisfied by member, not by
//!    declaration.** The spec says a `Duration` implements both and an
//!    `Instant` implements `Comparable`; `compareTo` and `toString` are
//!    registered and behave exactly as those interfaces require, but
//!    `nvs_types`' reserved interfaces carry no member signatures yet, so
//!    `$a < $b` is still refused and `$d->compareTo($e)` is the spelling that
//!    works. `"took " . $d` is not: ADR 0028 § 1's rendering is decided by the
//!    registered `toString` rather than by a declaration, through
//!    `nvs_types::expr::operators::require_stringable` where the operand's
//!    type names this class and `nvs_stdlib::instance`'s descriptor renderer
//!    where it names none.
//!
//! # What these members do with a qualifier
//!
//! ADR 0088 § 2's classification splits this module's `string` parameters in
//! two, and neither half is [`Qual::Contagious`] — which is unusual enough to
//! be worth the paragraph.
//!
//! * **Every pattern is a [`Qual::Sink`].** ADR 0063 R11's third grammar is
//!   the CLDR date pattern, and ADR 0088 § 1's corollary makes a grammar a
//!   sink wherever it is declared: the three `format` members' one parameter,
//!   and `Core\Time::parse`'s *second*. A pattern is an instruction to
//!   [`crate::cldr`], so a `tainted` one is refused rather than compiled.
//! * **Every parsed text is [`Qual::Neutral`]** — `Core\Time::fromIso`,
//!   `Core\Time::parse`'s first argument, `Duration::parse` and `Zone::of`.
//!   Each answers a value from a **closed space**: an instant, a civil
//!   date-time, a magnitude of nanoseconds, an entry of the IANA roster. No
//!   byte of the argument survives into any of them, and rendering one back
//!   goes through a pattern the *call site* wrote, so there is nothing for a
//!   qualifier to travel on. This is ADR 0024 § 2's "a checked conversion
//!   launders" reached at a member rather than at a cast, and it is the same
//!   judgement ADR 0102 § 5 makes when it narrows a route capture to a closed
//!   set with a type.
//!
//!   The line it draws is the one `Core\Bytes::at` is on the other side of:
//!   `at` answers a `uint` and is contagious anyway, because that `uint` *is*
//!   a byte of the subject and `fill`/`join` put the buffer back together out
//!   of those numbers. A parse's answer cannot be taken apart into the text it
//!   came from. The neighbouring class to be careful with is `Core\Uri`, whose
//!   `parse` keeps the host and the path as text and so is contagious.

use std::sync::OnceLock;

use jiff::civil::{self, Weekday};
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp, Zoned};
use nvs_runtime::host::Woken;
use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};
use nvs_syntax::duration;

use crate::registry::{
    Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy, Qual,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// `Core\Time\Duration`'s fully-qualified name, written once — [`DURATION`]
/// declares it and every [`CoreTy::Instance`] naming it resolves against
/// [`crate::registry::CLASSES`], so the two cannot drift apart.
///
/// `nvs_types::expr` reads this same constant for the type of an
/// [`nvs_syntax::ast::ExprKind::Duration`] literal, which is what makes ADR
/// 0070 § 2's "the suffix *is* the type" one fact rather than two spellings.
pub const DURATION_NAME: &str = r"Core\Time\Duration";

/// The symbol behind `Core\Time\Duration::nanoseconds`, which is also how a
/// duration **literal** reaches a value — `nvs_ir::lower` emits one
/// `InstKind::CoreCall` to it with the folded nanosecond count.
///
/// Named here rather than spelled in `nvs-ir` so that the registry row and the
/// literal cannot come to mean different things.
pub const FROM_NANOS_SYMBOL: &str = "nvs_core_time_duration_nanoseconds";

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
            doc: None,
        },
        CoreMethod {
            name: "microseconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_microseconds",
            doc: None,
        },
        CoreMethod {
            name: "milliseconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_milliseconds",
            doc: None,
        },
        CoreMethod {
            name: "seconds",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_seconds",
            doc: None,
        },
        CoreMethod {
            name: "minutes",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_minutes",
            doc: None,
        },
        CoreMethod {
            name: "hours",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_hours",
            doc: None,
        },
        CoreMethod {
            name: "days",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_days",
            doc: None,
        },
        CoreMethod {
            name: "weeks",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_weeks",
            doc: None,
        },
        CoreMethod {
            name: "parse",
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_parse",
            doc: None,
        },
    ],
    instance: &[
        CoreMethod {
            name: "toNanoseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_nanoseconds",
            doc: None,
        },
        CoreMethod {
            name: "toMicroseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_microseconds",
            doc: None,
        },
        CoreMethod {
            name: "toMilliseconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_milliseconds",
            doc: None,
        },
        CoreMethod {
            name: "toSeconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_seconds",
            doc: None,
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_plus",
            doc: None,
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_minus",
            doc: None,
        },
        CoreMethod {
            name: "multipliedBy",
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_multiplied_by",
            doc: None,
        },
        CoreMethod {
            name: "negated",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_negated",
            doc: None,
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_compare_to",
            doc: None,
        },
        CoreMethod {
            name: "toString",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_duration_to_string",
            doc: None,
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
/// One arm per class rather than one match over all of them: § 4 is one
/// domain but several classes, and a class's symbols stay beside the class.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    duration_address(symbol)
        .or_else(|| time_address(symbol))
        .or_else(|| instant_address(symbol))
        .or_else(|| datetime_address(symbol))
        .or_else(|| date_address(symbol))
        .or_else(|| time_of_day_address(symbol))
        .or_else(|| zone_address(symbol))
}

/// [`DURATION`]'s symbols.
fn duration_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        FROM_NANOS_SYMBOL => (nvs_core_time_duration_nanoseconds as *const ()).cast(),
        "nvs_core_time_duration_microseconds" => {
            (nvs_core_time_duration_microseconds as *const ()).cast()
        }
        "nvs_core_time_duration_milliseconds" => {
            (nvs_core_time_duration_milliseconds as *const ()).cast()
        }
        "nvs_core_time_duration_seconds" => (nvs_core_time_duration_seconds as *const ()).cast(),
        "nvs_core_time_duration_minutes" => (nvs_core_time_duration_minutes as *const ()).cast(),
        "nvs_core_time_duration_hours" => (nvs_core_time_duration_hours as *const ()).cast(),
        "nvs_core_time_duration_days" => (nvs_core_time_duration_days as *const ()).cast(),
        "nvs_core_time_duration_weeks" => (nvs_core_time_duration_weeks as *const ()).cast(),
        "nvs_core_time_duration_parse" => (nvs_core_time_duration_parse as *const ()).cast(),
        "nvs_core_time_duration_to_nanoseconds" => {
            (nvs_core_time_duration_to_nanoseconds as *const ()).cast()
        }
        "nvs_core_time_duration_to_microseconds" => {
            (nvs_core_time_duration_to_microseconds as *const ()).cast()
        }
        "nvs_core_time_duration_to_milliseconds" => {
            (nvs_core_time_duration_to_milliseconds as *const ()).cast()
        }
        "nvs_core_time_duration_to_seconds" => {
            (nvs_core_time_duration_to_seconds as *const ()).cast()
        }
        "nvs_core_time_duration_plus" => (nvs_core_time_duration_plus as *const ()).cast(),
        "nvs_core_time_duration_minus" => (nvs_core_time_duration_minus as *const ()).cast(),
        "nvs_core_time_duration_multiplied_by" => {
            (nvs_core_time_duration_multiplied_by as *const ()).cast()
        }
        "nvs_core_time_duration_negated" => (nvs_core_time_duration_negated as *const ()).cast(),
        "nvs_core_time_duration_compare_to" => {
            (nvs_core_time_duration_compare_to as *const ()).cast()
        }
        "nvs_core_time_duration_to_string" => {
            (nvs_core_time_duration_to_string as *const ()).cast()
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
/// A [`Fault::fatal`] naming the member — `nvs_types` has already checked the
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
pub(crate) fn nanos_of(args: &[Value], at: usize, member: &str) -> Result<i64, Fault> {
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

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::nanoseconds(int $n): Duration`.
    ///
    /// Also where a **literal** lands: ADR 0070 § 3 folds `1h30m` to its
    /// nanosecond count while compiling, and `nvs-ir` emits one call to this
    /// with that constant — so the literal and the constructor cannot produce
    /// different values.
    fn nvs_core_time_duration_nanoseconds(_ctx, args: [1]) {
        Ok(built(count(args, 0, "nanoseconds")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::microseconds(int $n): Duration`.
    fn nvs_core_time_duration_microseconds(_ctx, args: [1]) {
        scaled(args, "microseconds", 1_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::milliseconds(int $n): Duration`.
    fn nvs_core_time_duration_milliseconds(_ctx, args: [1]) {
        scaled(args, "milliseconds", 1_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::seconds(int $n): Duration`.
    fn nvs_core_time_duration_seconds(_ctx, args: [1]) {
        scaled(args, "seconds", 1_000_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::minutes(int $n): Duration`.
    fn nvs_core_time_duration_minutes(_ctx, args: [1]) {
        scaled(args, "minutes", 60 * 1_000_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::hours(int $n): Duration`.
    fn nvs_core_time_duration_hours(_ctx, args: [1]) {
        scaled(args, "hours", 60 * 60 * 1_000_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::days(int $n): Duration` — exactly 24 hours per
    /// day, never a calendar day (ADR 0070 § 1). A calendar step is
    /// `DateTime::plus($n, Unit::Day)`, which is a different type's member for
    /// exactly this reason.
    fn nvs_core_time_duration_days(_ctx, args: [1]) {
        scaled(args, "days", 24 * 60 * 60 * 1_000_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::weeks(int $n): Duration` — exactly 168 hours, on
    /// the same reasoning as `days`.
    fn nvs_core_time_duration_weeks(_ctx, args: [1]) {
        scaled(args, "weeks", 7 * 24 * 60 * 60 * 1_000_000_000)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time\Duration::parse(string $text): Duration` — ADR 0070 § 5's
    /// run-time entry point into the *same* grammar the lexer reads, so
    /// `Duration::parse("1h30m")` and the literal `1h30m` are one value.
    ///
    /// Throws on anything it does not accept, which is what makes it an
    /// [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
    /// launderer for a `tainted` config string — the qualifier half of that is
    /// still owed, and `crate::regex`'s gap 2 owns why nothing in
    /// [`crate::registry`] can state it yet.
    fn nvs_core_time_duration_parse(_ctx, args: [1]) {
        let text = text_of(args, 0, "Core\\Time\\Duration::parse")?;
        match duration::parse(text) {
            Ok(nanos) => Ok(built(nanos)),
            Err(err) => Err(Fault::thrown_as(
                ThrownClass::Parse,
                format!("Core\\Time\\Duration::parse(): {}", err.message()),
            )),
        }
    }
}

// ============================================================================
// The instance members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `$d->toNanoseconds(): int` — the whole state, exactly.
    fn nvs_core_time_duration_to_nanoseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toNanoseconds")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->toMicroseconds(): int`, truncating toward zero — `1500ns` is `1`
    /// and `-1500ns` is `-1`, which is what Rust's and PHP's integer division
    /// both already do, so nothing new has to be remembered.
    fn nvs_core_time_duration_to_microseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toMicroseconds")? / 1_000))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->toMilliseconds(): int`, truncating toward zero.
    fn nvs_core_time_duration_to_milliseconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toMilliseconds")? / 1_000_000))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->toSeconds(): int`, truncating toward zero.
    fn nvs_core_time_duration_to_seconds(_ctx, args: [1]) {
        Ok(Value::int(nanos_of(args, 0, "toSeconds")? / 1_000_000_000))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->plus(Duration $other): Duration` — a fresh value, since ADR 0063
    /// R3 makes every `Core` member pure.
    fn nvs_core_time_duration_plus(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "plus")?;
        let right = nanos_of(args, 1, "plus")?;
        left.checked_add(right)
            .map(built)
            .ok_or_else(|| overflowed("plus"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->minus(Duration $other): Duration` — the spelling ADR 0070 § 4
    /// gives a backwards step, since `-7d` does not parse.
    fn nvs_core_time_duration_minus(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "minus")?;
        let right = nanos_of(args, 1, "minus")?;
        left.checked_sub(right)
            .map(built)
            .ok_or_else(|| overflowed("minus"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->multipliedBy(int $factor): Duration`.
    fn nvs_core_time_duration_multiplied_by(_ctx, args: [2]) {
        let nanos = nanos_of(args, 0, "multipliedBy")?;
        let factor = count(args, 1, "multipliedBy")?;
        nanos
            .checked_mul(factor)
            .map(built)
            .ok_or_else(|| overflowed("multipliedBy"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->negated(): Duration`.
    ///
    /// The single value with no negation is `nanoseconds(i64::MIN)`, which
    /// throws rather than wrapping to itself — the one place this member can
    /// fail, and the reason it is written with `checked_neg` rather than `-`.
    fn nvs_core_time_duration_negated(_ctx, args: [1]) {
        nanos_of(args, 0, "negated")?
            .checked_neg()
            .map(built)
            .ok_or_else(|| overflowed("negated"))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->compareTo(Duration $other): int` — `Comparable`'s member
    /// ([ADR 0013](../../../../docs/adr/0013-comparable-interface.md)),
    /// answering the sign of `$d - $other` without the subtraction's overflow.
    fn nvs_core_time_duration_compare_to(_ctx, args: [2]) {
        let left = nanos_of(args, 0, "compareTo")?;
        let right = nanos_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->toString(): string` — `Stringable`'s member
    /// ([ADR 0028](../../../../docs/adr/0028-closing-the-remaining-magic-methods.md)),
    /// emitting ADR 0070 § 1's grammar so that a value round-trips through
    /// `parse` — over the durations that grammar can spell, which is the
    /// non-negative ones. This member is total and a negative duration is
    /// reachable through `minus` and `negated`, so it renders one with a
    /// leading `-` that ADR 0070 § 5 has `parse` refuse by name.
    fn nvs_core_time_duration_to_string(_ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "toString")?;
        Ok(Value::str(NvsStr::new(duration::render(nanos).as_bytes())))
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
/// [`nvs_core_time_zone_fixed`] one. The two cannot collide — no IANA
/// identifier begins with a sign — so [`resolve_zone`] tells them apart by the
/// first byte, and a `Zone` stays a value Novis can hold rather than a native
/// handle ([`crate::instance`] owns why that matters).
pub const ZONE: CoreClass = CoreClass {
    name: ZONE_NAME,
    methods: &[
        CoreMethod {
            name: "of",
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_of",
            doc: None,
        },
        CoreMethod {
            name: "fixed",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_fixed",
            doc: None,
        },
        CoreMethod {
            name: "system",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_system",
            doc: None,
        },
    ],
    instance: &[CoreMethod {
        name: "offsetAt",
        params: &[CoreTy::Instance(INSTANT_NAME)],
        defaults: &[],
        return_ty: CoreTy::Instance(DURATION_NAME),
        symbol: "nvs_core_time_zone_offset_at",
        doc: None,
    }],
    slots: &["id"],
    constants: &[CoreConst {
        name: "UTC",
        ty: CoreTy::Instance(ZONE_NAME),
        value: Const::Built {
            symbol: "nvs_core_time_zone_of",
            args: &[Const::Str("UTC")],
        },
    }],
};

/// [`ZONE`]'s one slot, by index.
const ZONE_ID_SLOT: usize = 0;

/// [`ZONE`]'s symbols.
fn zone_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_time_zone_of" => (nvs_core_time_zone_of as *const ()).cast(),
        "nvs_core_time_zone_fixed" => (nvs_core_time_zone_fixed as *const ()).cast(),
        "nvs_core_time_zone_system" => (nvs_core_time_zone_system as *const ()).cast(),
        "nvs_core_time_zone_offset_at" => (nvs_core_time_zone_offset_at as *const ()).cast(),
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
            symbol: "nvs_core_time_instant_in",
            doc: None,
        },
        CoreMethod {
            name: "toEpochSeconds",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_seconds",
            doc: None,
        },
        CoreMethod {
            name: "toEpochMillis",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_millis",
            doc: None,
        },
        CoreMethod {
            name: "toEpochMicros",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_micros",
            doc: None,
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_instant_plus",
            doc: None,
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_instant_minus",
            doc: None,
        },
        CoreMethod {
            name: "since",
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_instant_since",
            doc: None,
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_compare_to",
            doc: None,
        },
        CoreMethod {
            name: "toIso",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_instant_to_iso",
            doc: None,
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
        "nvs_core_time_instant_in" => (nvs_core_time_instant_in as *const ()).cast(),
        "nvs_core_time_instant_to_epoch_seconds" => {
            (nvs_core_time_instant_to_epoch_seconds as *const ()).cast()
        }
        "nvs_core_time_instant_to_epoch_millis" => {
            (nvs_core_time_instant_to_epoch_millis as *const ()).cast()
        }
        "nvs_core_time_instant_to_epoch_micros" => {
            (nvs_core_time_instant_to_epoch_micros as *const ()).cast()
        }
        "nvs_core_time_instant_plus" => (nvs_core_time_instant_plus as *const ()).cast(),
        "nvs_core_time_instant_minus" => (nvs_core_time_instant_minus as *const ()).cast(),
        "nvs_core_time_instant_since" => (nvs_core_time_instant_since as *const ()).cast(),
        "nvs_core_time_instant_compare_to" => {
            (nvs_core_time_instant_compare_to as *const ()).cast()
        }
        "nvs_core_time_instant_to_iso" => (nvs_core_time_instant_to_iso as *const ()).cast(),
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
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_datetime_format",
            doc: None,
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_plus",
            doc: None,
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_minus",
            doc: None,
        },
        CoreMethod {
            name: "next",
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_next",
            doc: None,
        },
        CoreMethod {
            name: "previous",
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_previous",
            doc: None,
        },
        CoreMethod {
            name: "with",
            params: &[CoreTy::Options(WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_with",
            doc: None,
        },
        CoreMethod {
            name: "withTime",
            params: &[CoreTy::Instance(TIME_OF_DAY_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_with_time",
            doc: None,
        },
        CoreMethod {
            name: "startOf",
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_start_of",
            doc: None,
        },
        CoreMethod {
            name: "endOf",
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_end_of",
            doc: None,
        },
        CoreMethod {
            name: "difference",
            params: &[CoreTy::Instance(DATETIME_NAME), CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_datetime_difference",
            doc: None,
        },
        CoreMethod {
            name: "toInstant",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_datetime_to_instant",
            doc: None,
        },
        CoreMethod {
            name: "date",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_datetime_date",
            doc: None,
        },
        CoreMethod {
            name: "timeOfDay",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_datetime_time_of_day",
            doc: None,
        },
        CoreMethod {
            name: "zone",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_datetime_zone",
            doc: None,
        },
        CoreMethod {
            name: "weekday",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(WEEKDAY.name),
            symbol: "nvs_core_time_datetime_weekday",
            doc: None,
        },
        CoreMethod {
            name: "dayOfYear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_time_datetime_day_of_year",
            doc: None,
        },
        CoreMethod {
            name: "isLeapYear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_time_datetime_is_leap_year",
            doc: None,
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
        "nvs_core_time_datetime_format" => (nvs_core_time_datetime_format as *const ()).cast(),
        "nvs_core_time_datetime_plus" => (nvs_core_time_datetime_plus as *const ()).cast(),
        "nvs_core_time_datetime_minus" => (nvs_core_time_datetime_minus as *const ()).cast(),
        "nvs_core_time_datetime_next" => (nvs_core_time_datetime_next as *const ()).cast(),
        "nvs_core_time_datetime_previous" => (nvs_core_time_datetime_previous as *const ()).cast(),
        "nvs_core_time_datetime_with" => (nvs_core_time_datetime_with as *const ()).cast(),
        "nvs_core_time_datetime_with_time" => {
            (nvs_core_time_datetime_with_time as *const ()).cast()
        }
        "nvs_core_time_datetime_start_of" => (nvs_core_time_datetime_start_of as *const ()).cast(),
        "nvs_core_time_datetime_end_of" => (nvs_core_time_datetime_end_of as *const ()).cast(),
        "nvs_core_time_datetime_difference" => {
            (nvs_core_time_datetime_difference as *const ()).cast()
        }
        "nvs_core_time_datetime_to_instant" => {
            (nvs_core_time_datetime_to_instant as *const ()).cast()
        }
        "nvs_core_time_datetime_date" => (nvs_core_time_datetime_date as *const ()).cast(),
        "nvs_core_time_datetime_time_of_day" => {
            (nvs_core_time_datetime_time_of_day as *const ()).cast()
        }
        "nvs_core_time_datetime_zone" => (nvs_core_time_datetime_zone as *const ()).cast(),
        "nvs_core_time_datetime_weekday" => (nvs_core_time_datetime_weekday as *const ()).cast(),
        "nvs_core_time_datetime_day_of_year" => {
            (nvs_core_time_datetime_day_of_year as *const ()).cast()
        }
        "nvs_core_time_datetime_is_leap_year" => {
            (nvs_core_time_datetime_is_leap_year as *const ()).cast()
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
/// classes above are members of.
pub const TIME: CoreClass = CoreClass {
    name: TIME_NAME,
    methods: &[
        CoreMethod {
            name: "now",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_now",
            doc: None,
        },
        CoreMethod {
            name: "monotonic",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_monotonic",
            doc: None,
        },
        CoreMethod {
            name: "sleep",
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_time_sleep",
            doc: None,
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
            symbol: "nvs_core_time_from_epoch",
            doc: None,
        },
        CoreMethod {
            name: "fromIso",
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_from_iso",
            doc: None,
        },
        CoreMethod {
            name: "parse",
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Sink),
                CoreTy::Instance(ZONE_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_parse",
            doc: None,
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
            symbol: "nvs_core_time_at",
            doc: None,
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// [`TIME`]'s symbols.
fn time_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_time_now" => (nvs_core_time_now as *const ()).cast(),
        "nvs_core_time_monotonic" => (nvs_core_time_monotonic as *const ()).cast(),
        "nvs_core_time_sleep" => (nvs_core_time_sleep as *const ()).cast(),
        "nvs_core_time_from_epoch" => (nvs_core_time_from_epoch as *const ()).cast(),
        "nvs_core_time_from_iso" => (nvs_core_time_from_iso as *const ()).cast(),
        "nvs_core_time_parse" => (nvs_core_time_parse as *const ()).cast(),
        "nvs_core_time_at" => (nvs_core_time_at as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// `Core\Time\Date` — registration
// ============================================================================

/// `Core\Time\Date`'s fully-qualified name, written once, for
/// [`DURATION_NAME`]'s reason.
pub const DATE_NAME: &str = r"Core\Time\Date";

/// The `{year?, month?, day?}` bag `$d->with(…)` takes — [`WITH_OPTIONS`]
/// without the four fields a zone-free date does not carry.
///
/// A separate roster rather than a subslice of that one because the two are
/// the same *shape* by coincidence rather than by rule: § 4 gives `DateTime`
/// seven fields and `Date` three, and a `Date` that grew an `hour` option
/// would be a `DateTime`.
const DATE_WITH_OPTIONS: &[CoreOption] = &[
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
];

/// Spec § 4's `Core\Time\Date` — the zone-free calendar half of a
/// [`DATETIME`], with the same `plus`/`minus`/`with`/`compareTo`/`format`
/// shape and the constructor `Date::at`.
///
/// Three slots, one per civil field, which is the opposite of the choice
/// [`DATETIME`] documents — and for the reason that made that one necessary.
/// A `DateTime` stores its instant because a civil triple could hold 02:30 on
/// a spring-forward morning, a time that does not exist and that every member
/// would then have to have an opinion about. A `Date` has no zone, so it has
/// no such value: the only triples that are not a real date are ones
/// [`jiff::civil::Date::new`] already rejects where the value is built, and
/// nothing but a constructor ever writes these slots.
///
/// **What it spends:** 48 bytes of slots per value, charged to the request
/// that produced it, against the 16 a single day-count slot would take. The
/// three fields are what every member reads, so a count would be a conversion
/// on each side of every call to save two words — [AGENTS.md](../../../../AGENTS.md)'s
/// priority ordering spends memory on simplicity, not the reverse.
pub const DATE: CoreClass = CoreClass {
    name: DATE_NAME,
    methods: &[CoreMethod {
        name: "at",
        params: &[CoreTy::Int, CoreTy::Uint, CoreTy::Uint],
        defaults: &[],
        return_ty: CoreTy::Instance(DATE_NAME),
        symbol: "nvs_core_time_date_at",
        doc: None,
    }],
    instance: &[
        CoreMethod {
            name: "format",
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_date_format",
            doc: None,
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_plus",
            doc: None,
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_minus",
            doc: None,
        },
        CoreMethod {
            name: "with",
            params: &[CoreTy::Options(DATE_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_with",
            doc: None,
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(DATE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_date_compare_to",
            doc: None,
        },
    ],
    slots: &["year", "month", "day"],
    constants: &[],
};

/// [`DATE`]'s slots, by index.
const DATE_YEAR_SLOT: usize = 0;
/// See [`DATE_YEAR_SLOT`].
const DATE_MONTH_SLOT: usize = 1;
/// See [`DATE_YEAR_SLOT`].
const DATE_DAY_SLOT: usize = 2;

/// [`DATE`]'s symbols.
fn date_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_time_date_at" => (nvs_core_time_date_at as *const ()).cast(),
        "nvs_core_time_date_format" => (nvs_core_time_date_format as *const ()).cast(),
        "nvs_core_time_date_plus" => (nvs_core_time_date_plus as *const ()).cast(),
        "nvs_core_time_date_minus" => (nvs_core_time_date_minus as *const ()).cast(),
        "nvs_core_time_date_with" => (nvs_core_time_date_with as *const ()).cast(),
        "nvs_core_time_date_compare_to" => (nvs_core_time_date_compare_to as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// `Core\Time\TimeOfDay` — registration
// ============================================================================

/// `Core\Time\TimeOfDay`'s fully-qualified name, written once, for
/// [`DURATION_NAME`]'s reason.
pub const TIME_OF_DAY_NAME: &str = r"Core\Time\TimeOfDay";

/// The `{second?: uint, nanos?: uint}` bag `TimeOfDay::at` takes — the two
/// fields a wall clock usually leaves off, defaulting to zero.
///
/// [`Const::Uint`] rather than [`Const::Null`] here, unlike
/// [`TIME_OF_DAY_WITH_OPTIONS`], because a *constructor* has a right answer
/// for an omitted field and `with` does not: `10:30` means second zero, while
/// `$t->with({})` means leave every field alone.
const AT_OPTIONS: &[CoreOption] = &[
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
];

/// The `{hour?, minute?, second?, nanos?}` bag `$t->with(…)` takes —
/// [`WITH_OPTIONS`] without the three calendar fields.
const TIME_OF_DAY_WITH_OPTIONS: &[CoreOption] = &[
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

/// Spec § 4's `Core\Time\TimeOfDay` — the zone-free clock half of a
/// [`DATETIME`], and [`DATE`]'s opposite number in every way including its
/// shape.
///
/// Four slots, one per civil field, for [`DATE`]'s reason: nothing but a
/// constructor writes them, and [`jiff::civil::Time::new`] has already refused
/// every combination that is not a time.
///
/// **What it spends:** 64 bytes of slots per value, charged to the request
/// that produced it.
pub const TIME_OF_DAY: CoreClass = CoreClass {
    name: TIME_OF_DAY_NAME,
    methods: &[CoreMethod {
        name: "at",
        params: &[CoreTy::Uint, CoreTy::Uint, CoreTy::Options(AT_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
        symbol: "nvs_core_time_of_day_at",
        doc: None,
    }],
    instance: &[
        CoreMethod {
            name: "format",
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_of_day_format",
            doc: None,
        },
        CoreMethod {
            name: "plus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_plus",
            doc: None,
        },
        CoreMethod {
            name: "minus",
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_minus",
            doc: None,
        },
        CoreMethod {
            name: "with",
            params: &[CoreTy::Options(TIME_OF_DAY_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_with",
            doc: None,
        },
        CoreMethod {
            name: "compareTo",
            params: &[CoreTy::Instance(TIME_OF_DAY_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_of_day_compare_to",
            doc: None,
        },
    ],
    slots: &["hour", "minute", "second", "nanos"],
    constants: &[],
};

/// [`TIME_OF_DAY`]'s slots, by index.
const CLOCK_HOUR_SLOT: usize = 0;
/// See [`CLOCK_HOUR_SLOT`].
const CLOCK_MINUTE_SLOT: usize = 1;
/// See [`CLOCK_HOUR_SLOT`].
const CLOCK_SECOND_SLOT: usize = 2;
/// See [`CLOCK_HOUR_SLOT`].
const CLOCK_NANOS_SLOT: usize = 3;

/// [`TIME_OF_DAY`]'s symbols.
fn time_of_day_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_time_of_day_at" => (nvs_core_time_of_day_at as *const ()).cast(),
        "nvs_core_time_of_day_format" => (nvs_core_time_of_day_format as *const ()).cast(),
        "nvs_core_time_of_day_plus" => (nvs_core_time_of_day_plus as *const ()).cast(),
        "nvs_core_time_of_day_minus" => (nvs_core_time_of_day_minus as *const ()).cast(),
        "nvs_core_time_of_day_with" => (nvs_core_time_of_day_with as *const ()).cast(),
        "nvs_core_time_of_day_compare_to" => (nvs_core_time_of_day_compare_to as *const ()).cast(),
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
    // The three guards below are unreachable from source: `Core\Time\Instant`
    // is a `Core`-owned class, so `new Core\Time\Instant()` is `E0405: has no
    // member named constructor` at the checker and nothing but
    // [`instant_built`] ever writes these slots — an `int` second, and a
    // nanosecond a live `Timestamp` already bounded to 0..1_000_000_000.
    let seconds = crate::instance::slot(object, INSTANT_SECONDS_SLOT)
        .as_int()
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Instant::{member} found a non-`int` `seconds` slot"
            ))
        })?;
    // Unreachable from source for the reason above.
    let nanos = crate::instance::slot(object, INSTANT_NANOS_SLOT)
        .as_int()
        .and_then(|held| i32::try_from(held).ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Instant::{member} found an out-of-range `nanos` slot"
            ))
        })?;
    // Unreachable from source for the reason above: the pair being recombined
    // here is the pair a live `Timestamp` was taken apart into.
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
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives, and
/// only that one: a `string` is guaranteed-valid UTF-8 (ADR 0009), and the tag
/// [`Value::as_text`] checks *is* that guarantee, so there is nothing to
/// re-derive here — `crate::str`'s own `text` states what doing it anyway costs.
fn text_of<'a>(args: &'a [Value], at: usize, member: &str) -> Result<&'a str, Fault> {
    args[at].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "{member} expected a `string`, got tag {}",
            args[at].tag_byte()
        ))
    })
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
    crate::instance::build(&ZONE, [Value::str(NvsStr::new(id.as_bytes()))])
}

/// The [`TimeZone`] the `Zone` in argument slot `at` names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a string, and a
/// [`Fault::thrown`] for an id the database no longer has — which only a
/// database swapped under a live process can produce, since
/// [`nvs_core_time_zone_of`] resolved it once already.
fn zone_of(args: &[Value], at: usize, member: &str) -> Result<TimeZone, Fault> {
    let object = crate::instance::receiver(args[at], &ZONE, member)?;
    let held = crate::instance::slot(object, ZONE_ID_SLOT);
    // A `string` is guaranteed-valid UTF-8 (ADR 0009) and the tag is what says
    // so, and this crate wrote this slot — so the tag check is the whole read.
    let id = held.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Time\\Zone::{member} found a non-`string` `id` slot"
        ))
    })?;
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
/// [`nvs_core_time_monotonic`] call.
///
/// Process-wide rather than per-core so that two cores' readings are
/// differences on one timeline, which is the only thing a monotonic clock is
/// for; written once and read thereafter, so the sharing costs no
/// synchronisation past the first call.
static MONOTONIC_ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();

nvs_runtime::nvs_helper! {
    /// `Core\Time::now(): Instant` — the wall clock, which replaces `time`,
    /// `microtime` and `date_create` at once.
    fn nvs_core_time_now(_ctx, args: [0]) {
        let _ = args;
        Ok(instant_built(Timestamp::now()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::monotonic(): Duration` — for measuring, never for
    /// wall-clock, which is why it answers with a `Duration` and not an
    /// `Instant`: the value has no meaning except against another reading of
    /// the same clock.
    fn nvs_core_time_monotonic(_ctx, args: [0]) {
        let _ = args;
        let origin = MONOTONIC_ORIGIN.get_or_init(std::time::Instant::now);
        Ok(built(
            i64::try_from(origin.elapsed().as_nanos()).unwrap_or(i64::MAX),
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::sleep(Duration $d): void`, replacing `sleep`, `usleep`,
    /// `time_nanosleep` and `time_sleep_until` — one member, because the four
    /// differ only in the unit a `Duration` already carries.
    ///
    /// A negative or zero duration returns at once rather than throwing:
    /// "sleep until a moment already past" is a wait of no time, which is what
    /// every caller computing a deadline wants.
    ///
    /// **Parks the task rather than blocking the core** — the wait goes to
    /// [`nvs_runtime::host::Host::sleep`], which is `nvs-host`'s reactor
    /// arming this task's own deadline, so a neighbour pinned to the same core
    /// runs while this one waits (ADR 0106 § 6). It is also what makes a
    /// `Core\Task::all` under a `limit` observable at all: with a blocking
    /// sleep no two children ever overlap.
    ///
    /// [`Woken::Cancelled`] is the group cancelling this child mid-sleep. The
    /// member stops the request there and then, through [`Ctx::cancel`], which
    /// is ADR 0072 § 5's teardown reached by ADR 0002's return status — the
    /// only way it can be reached from a frame that is `extern "C"`.
    ///
    /// With no host on the thread the wait still has to happen, and blocking is
    /// the right answer: nothing else is running on this core.
    fn nvs_core_time_sleep(ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "sleep")?;
        if nanos > 0 {
            let duration = std::time::Duration::from_nanos(nanos.unsigned_abs());
            match nvs_runtime::host::with_current(|host| host.sleep(duration)) {
                Some(Woken::Cancelled) => return Err(ctx.cancel()),
                Some(Woken::Elapsed) => {}
                None => std::thread::sleep(duration),
            }
        }
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::fromEpoch(int $seconds, {nanos?: uint}): Instant`.
    ///
    /// The `nanos` option is added *after* the second, rather than written
    /// into the slot beside it, so that a negative second and a positive
    /// `nanos` mean what a reader expects — one nanosecond after
    /// `-1`, not one before it.
    fn nvs_core_time_from_epoch(_ctx, args: [2]) {
        let seconds = count(args, 0, "fromEpoch")?;
        // Unreachable from source: `nanos` is a `CoreTy::Uint` option in
        // `CLASS` above, so anything else is `E0401: expected uint, found
        // mixed` — reported inside the `{nanos: …}` shape literal at the call,
        // which is where an option is written and where it was probed.
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

nvs_runtime::nvs_helper! {
    /// `Core\Time::fromIso(string $text): Instant` — an ISO-8601 timestamp
    /// that carries its own offset, which is the whole of what `strtotime`
    /// replaces here. A relative expression is not accepted in either half
    /// (§ 4), and a *civil* time with no offset is `Time::parse`, which needs
    /// a zone.
    ///
    /// The refusal is a `ParseError` on the same rule
    /// [`nvs_core_time_parse`] splits its two classes by: this member's one
    /// argument is text, which arrives from somewhere else, so a text that
    /// does not spell a timestamp is the input's failure and not the call
    /// site's. The sentence past the member's own name is `jiff`'s, since it
    /// says which component it stopped at and nothing here could say it
    /// better.
    fn nvs_core_time_from_iso(_ctx, args: [1]) {
        let text = text_of(args, 0, "Core\\Time::fromIso")?;
        text.parse::<Timestamp>()
            .map(instant_built)
            .map_err(|err| {
                Fault::thrown_as(ThrownClass::Parse, format!("Core\\Time::fromIso(): {err}"))
            })
    }
}

// ============================================================================
// `Core\Time\Instant`'s members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `$i->toEpochSeconds(): int` — replaces `getTimestamp` and `date("U")`.
    fn nvs_core_time_instant_to_epoch_seconds(_ctx, args: [1]) {
        Ok(Value::int(instant_of(args, 0, "toEpochSeconds")?.as_second()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->toEpochMillis(): int` — one of `microtime(true)`'s two halves,
    /// as an exact integer rather than a `float` that loses the subsecond
    /// digits it was asked for.
    fn nvs_core_time_instant_to_epoch_millis(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `$i->toEpochMicros(): int` — `microtime`'s other half.
    fn nvs_core_time_instant_to_epoch_micros(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `$i->plus(Duration $d): Instant` — the exact arithmetic of § 4's two,
    /// so it crosses a DST boundary without noticing one. A calendar step is
    /// `DateTime::plus($n, Unit::Day)`.
    fn nvs_core_time_instant_plus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "plus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "plus")?);
        at.checked_add(by)
            .map(instant_built)
            .map_err(|err| out_of_range(r"Core\Time\Instant::plus", &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->minus(Duration $d): Instant`.
    fn nvs_core_time_instant_minus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "minus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "minus")?);
        at.checked_sub(by)
            .map(instant_built)
            .map_err(|err| out_of_range(r"Core\Time\Instant::minus", &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->since(Instant $earlier): Duration` — replaces `date_diff` and
    /// `DateInterval` arithmetic, with none of that type's "1 month" ambiguity
    /// because the answer is an exact count.
    ///
    /// Negative where `$earlier` is in fact later, which is what makes it the
    /// inverse of `plus` rather than an absolute distance.
    fn nvs_core_time_instant_since(_ctx, args: [2]) {
        let later = instant_of(args, 0, "since")?;
        let earlier = instant_of(args, 1, "since")?;
        Ok(built(nanos_between(later, earlier, "since")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->compareTo(Instant $other): int` — `Comparable`'s member
    /// ([ADR 0013](../../../../docs/adr/0013-comparable-interface.md)).
    fn nvs_core_time_instant_compare_to(_ctx, args: [2]) {
        let left = instant_of(args, 0, "compareTo")?;
        let right = instant_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->toIso(): string` — RFC 3339 in UTC, which is `date(DATE_ATOM)`'s
    /// replacement and the one rendering that needs no zone argument.
    fn nvs_core_time_instant_to_iso(_ctx, args: [1]) {
        let at = instant_of(args, 0, "toIso")?;
        Ok(Value::str(NvsStr::new(at.to_string().as_bytes())))
    }
}

// ============================================================================
// `Core\Time\Zone`'s members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Time\Zone::of(string $id): Zone` — an IANA identifier, replacing
    /// `DateTimeZone`. Throws on an unknown one rather than falling back to
    /// UTC, which is PHP's behaviour and the reason a mistyped zone there is a
    /// silent wrong answer.
    fn nvs_core_time_zone_of(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Time\Zone::fixed(Duration $offset): Zone` — for a timestamp that
    /// carries an offset instead of a region, which is every RFC 3339 string
    /// and no IANA identifier.
    ///
    /// A whole number of seconds, within the ±25:59:59 the IANA format itself
    /// allows; anything else throws rather than rounding.
    fn nvs_core_time_zone_fixed(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Time\Zone::system(): Zone` — the host's configured zone,
    /// replacing `date_default_timezone_get`.
    ///
    /// **Not** an ambient default (§ 4): it is an ordinary value a program
    /// asks for and then passes explicitly, so a call site still names the
    /// zone it converts in. There is no `date_default_timezone_set`.
    fn nvs_core_time_zone_system(_ctx, args: [0]) {
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

nvs_runtime::nvs_helper! {
    /// `$z->offsetAt(Instant $i): Duration` — replaces `getOffset`.
    ///
    /// Takes the instant because an offset is not a property of a zone:
    /// `Europe/Berlin` is `+01:00` in January and `+02:00` in July, and a
    /// member that did not ask would have to pick one silently.
    fn nvs_core_time_zone_offset_at(_ctx, args: [2]) {
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
            Value::str(NvsStr::new(id.as_bytes())),
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
    let id = held.as_text().ok_or_else(|| {
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

nvs_runtime::nvs_helper! {
    /// `$d->format(string $pattern): string` — CLDR patterns
    /// ([`crate::cldr`]), replacing `date`, `gmdate`, `idate`, `strftime` and
    /// `date_format` at once.
    ///
    /// The pattern is compiled per call; ADR 0057 is what moves that to
    /// compile time for a literal one, and [`crate::cldr`]'s gap 1 owns it.
    fn nvs_core_time_datetime_format(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "format")?;
        let pattern = text_of(args, 1, "Core\\Time\\DateTime::format")?;
        let pieces = crate::cldr::compile(pattern)
            .map_err(|why| Fault::thrown(format!("Core\\Time\\DateTime::format(): {why}")))?;
        Ok(Value::str(NvsStr::new(
            crate::cldr::render(&pieces, &at).as_bytes(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->plus(int $count, Unit $unit): DateTime` — § 4's *calendar* half,
    /// so adding `1, Unit::Month` lands on the same day-of-month clamped to
    /// the month's length, and crossing a DST boundary makes a 23- or 25-hour
    /// day. The exact half is `$d->toInstant()->plus(24h)`.
    fn nvs_core_time_datetime_plus(_ctx, args: [3]) {
        stepped(args, "plus", 1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->minus(int $count, Unit $unit): DateTime`.
    fn nvs_core_time_datetime_minus(_ctx, args: [3]) {
        stepped(args, "minus", -1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->next(Weekday $w): DateTime` — the nearest **strictly later** day
    /// with that weekday, time-of-day preserved. Replaces
    /// `strtotime("next monday")`.
    fn nvs_core_time_datetime_next(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "next")?;
        let weekday = weekday_of(args, 1, "Core\\Time\\DateTime::next")?;
        at.nth_weekday(1, weekday)
            .map(|found| datetime_built(&found))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::next", &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->previous(Weekday $w): DateTime` — [`nvs_core_time_datetime_next`]
    /// backwards, and strictly earlier for the same reason.
    fn nvs_core_time_datetime_previous(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "previous")?;
        let weekday = weekday_of(args, 1, "Core\\Time\\DateTime::previous")?;
        at.nth_weekday(-1, weekday)
            .map(|found| datetime_built(&found))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::previous", &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->with({year?, month?, day?, hour?, minute?, second?, nanos?}):
    /// DateTime` — replaces `setDate`, `setTime` and `setISODate`.
    ///
    /// An omitted option arrives as `Tag::Null` and leaves its field alone;
    /// see [`WITH_OPTIONS`] for why that is a tag rather than a sentinel.
    fn nvs_core_time_datetime_with(_ctx, args: [8]) {
        let at = zoned_of(args, 0, "with")?;
        let mut building = at.with();
        if let Some(year) = optional(args, 1, r"Core\Time\DateTime::with", "year")? {
            building = building.year(
                i16::try_from(year).map_err(|_| {
                    out_of_range(r"Core\Time\DateTime::with", "`year` is outside -9999..=9999")
                })?,
            );
        }
        for (index, name) in [(2, "month"), (3, "day"), (4, "hour"), (5, "minute"), (6, "second")] {
            let Some(value) = optional(args, index, r"Core\Time\DateTime::with", name)? else {
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
        if let Some(nanos) = optional(args, 7, r"Core\Time\DateTime::with", "nanos")? {
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

nvs_runtime::nvs_helper! {
    /// `$d->withTime(TimeOfDay $t): DateTime` — spec § 4's "common half of
    /// `with`, spelled as the operation it is".
    ///
    /// The same body as [`nvs_core_time_datetime_with`] reached from a
    /// component view instead of an options bag, and the one call of the two
    /// that cannot be partial: a `TimeOfDay` carries all four clock fields, so
    /// `nanos` is replaced as surely as `hour` is and there is no
    /// omitted-option case to leave alone. The date and the zone are what stay,
    /// which is why this is not `Core\Time::at` with three fields copied over.
    ///
    /// Landing in a DST gap resolves the way every other `DateTime` build
    /// does — `jiff`'s compatible disambiguation, shifting forward by the gap
    /// — rather than throwing, since the civil time the program named is the
    /// one the zone skipped and the next real instant is what it meant.
    fn nvs_core_time_datetime_with_time(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "withTime")?;
        let time = clock_of(args, 1, "withTime")?;
        at.with()
            .time(time)
            .build()
            .map(|built| datetime_built(&built))
            .map_err(|err| out_of_range(r"Core\Time\DateTime::withTime", &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->startOf(Unit $u): DateTime`.
    fn nvs_core_time_datetime_start_of(_ctx, args: [2]) {
        let at = zoned_of(args, 0, "startOf")?;
        let (unit, scale) = unit_of(args, 1, "Core\\Time\\DateTime::startOf")?;
        Ok(datetime_built(&floored(&at, unit, scale, "startOf")?))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->endOf(Unit $u): DateTime` — the **last** instant of the unit,
    /// which is one nanosecond before the next one starts.
    ///
    /// A distinct operation from `startOf` at a DST boundary, which is § 4's
    /// stated reason both exist: the two are not a fixed distance apart on a
    /// day the zone lengthens or shortens.
    fn nvs_core_time_datetime_end_of(_ctx, args: [2]) {
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

nvs_runtime::nvs_helper! {
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
    fn nvs_core_time_datetime_difference(_ctx, args: [3]) {
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

nvs_runtime::nvs_helper! {
    /// `$d->toInstant(): Instant` — free, since a `DateTime` already holds
    /// one ([`DATETIME`]).
    fn nvs_core_time_datetime_to_instant(_ctx, args: [1]) {
        Ok(instant_built(zoned_of(args, 0, "toInstant")?.timestamp()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->date(): Date` — § 4's first component view, and the one that
    /// drops the zone as well as the time: the civil date this value reads as
    /// **where it is**, which is why a zone conversion first
    /// (`$d->toInstant()->in($z)->date()`) can answer a different day.
    fn nvs_core_time_datetime_date(_ctx, args: [1]) {
        Ok(date_built(zoned_of(args, 0, "date")?.datetime().date()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->timeOfDay(): TimeOfDay` — the other component view, and the one
    /// that is a wall clock reading rather than a point on any timeline: two
    /// zones can read `09:00` at once, which is exactly why it carries
    /// neither the date nor the zone.
    fn nvs_core_time_datetime_time_of_day(_ctx, args: [1]) {
        Ok(clock_built(zoned_of(args, 0, "timeOfDay")?.datetime().time()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->zone(): Zone` — the zone this civil time is in, which is never
    /// ambient and so is always one the program named.
    fn nvs_core_time_datetime_zone(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "zone")?;
        Ok(match at.time_zone().iana_name() {
            Some(name) => zone_built(name),
            None => zone_built(&render_offset(at.offset().seconds())),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->weekday(): Weekday` — replaces `date("N")`, as the enum case
    /// rather than the number.
    fn nvs_core_time_datetime_weekday(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "weekday")?;
        Ok(Value::int(i64::from(
            at.weekday().to_monday_zero_offset(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->dayOfYear(): uint` — replaces `date("z")`, one-based where PHP's
    /// is zero-based, because every other day count in § 4 is one-based too.
    fn nvs_core_time_datetime_day_of_year(_ctx, args: [1]) {
        let at = zoned_of(args, 0, "dayOfYear")?;
        Ok(Value::uint(at.day_of_year().unsigned_abs().into()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->isLeapYear(): bool` — replaces `date("L")` and `checkdate`'s year
    /// half; the day half has no equivalent because an invalid date throws
    /// where it is built.
    fn nvs_core_time_datetime_is_leap_year(_ctx, args: [1]) {
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
/// [`count`] gives. `member` is the whole `Core\…::name` label, since both
/// `with` members reach here.
fn optional(args: &[Value], at: usize, member: &str, option: &str) -> Result<Option<i64>, Fault> {
    if args[at].tag() == Some(nvs_runtime::Tag::Null) {
        return Ok(None);
    }
    args[at]
        .as_int()
        .or_else(|| args[at].as_uint().and_then(|held| i64::try_from(held).ok()))
        .map(Some)
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a number for `{option}`, got tag {}",
                args[at].tag_byte()
            ))
        })
}

nvs_runtime::nvs_helper! {
    /// `$i->in(Zone $zone): DateTime` — the only instant→calendar conversion
    /// there is, and the reason § 4 needs no ambient timezone.
    fn nvs_core_time_instant_in(_ctx, args: [2]) {
        let at = instant_of(args, 0, "in")?;
        let zone = zone_of(args, 1, "in")?;
        Ok(datetime_built(&Zoned::new(at, zone)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::parse(string $text, string $format, Zone $zone): DateTime`
    /// — `DateTime::createFromFormat` and `strptime`, over [`crate::cldr`]'s
    /// patterns rather than PHP's letters.
    ///
    /// The zone is the third argument rather than something the pattern can
    /// name, which is why a zonal field in the pattern is refused: two answers
    /// for one question is what ADR 0063 R20 leaves no room for. That refusal
    /// is [`crate::cldr::civil_fields_only`]'s and is made against the compiled
    /// *pattern*, before a byte of the text is read, which is what puts it on
    /// the `LogicError` side of the split below rather than on the
    /// `ParseError` one — a well-formed offset in the text is not the input
    /// failing to match.
    fn nvs_core_time_parse(_ctx, args: [3]) {
        let text = text_of(args, 0, "Core\\Time::parse")?;
        let pattern = text_of(args, 1, "Core\\Time::parse")?;
        let zone = zone_of(args, 2, "parse")?;
        // The two failures are different spec § 10 classes on purpose: a
        // pattern this call site wrote wrongly is a bug in the program, while
        // text that does not match a well-formed pattern is exactly "input did
        // not match a format this code declared".
        let pieces = crate::cldr::compile(pattern)
            .and_then(|pieces| crate::cldr::civil_fields_only(&pieces).map(|()| pieces))
            .map_err(|why| {
                Fault::thrown_as(ThrownClass::Logic, format!("Core\\Time::parse(): {why}"))
            })?;
        crate::cldr::read(&pieces, text, &zone)
            .map(|at| datetime_built(&at))
            .map_err(|why| {
                Fault::thrown_as(ThrownClass::Parse, format!("Core\\Time::parse(): {why}"))
            })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::at(int $year, uint $month, uint $day, Zone $zone,
    /// {hour?, minute?, second?, nanos?}): DateTime` — replaces `mktime`,
    /// `gmmktime` and `DateTime::setDate`.
    ///
    /// A civil time the zone skips over — 02:30 on a spring-forward morning —
    /// resolves forward rather than throwing, which is [`jiff`]'s
    /// *compatible* disambiguation and the rule every other language's
    /// calendar library settled on. There is no `checkdate`: a date that does
    /// not exist at all throws here.
    fn nvs_core_time_at(_ctx, args: [8]) {
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
            // Unreachable from source: every slot this loop reads is a
            // `CoreTy::Uint` parameter or option in `CLASS` above, so anything
            // else is `E0401: expected uint, found mixed` at the checker.
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

// ============================================================================
// `Core\Time\Date` — reading, building and the calendar step
// ============================================================================

/// A fresh `Date` at `at`.
fn date_built(at: civil::Date) -> Value {
    crate::instance::build(
        &DATE,
        [
            Value::int(i64::from(at.year())),
            Value::int(i64::from(at.month())),
            Value::int(i64::from(at.day())),
        ],
    )
}

/// The [`civil::Date`] the `Date` in argument slot `at` holds — slot 0 for a
/// receiver, slot 1 for `compareTo`'s parameter.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives: the
/// three slots are written by a constructor that already went through
/// [`civil::Date::new`], so a triple that does not read back is compiled code
/// disagreeing with [`DATE`]'s layout.
fn date_of(args: &[Value], at: usize, member: &str) -> Result<civil::Date, Fault> {
    let object = crate::instance::receiver(args[at], &DATE, member)?;
    let read = |index: usize, what: &str| {
        crate::instance::slot(object, index)
            .as_int()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Time\\Date::{member} found a non-`int` `{what}` slot"
                ))
            })
    };
    let year = read(DATE_YEAR_SLOT, "year")?;
    let month = read(DATE_MONTH_SLOT, "month")?;
    let day = read(DATE_DAY_SLOT, "day")?;
    parts(year, month, day)
        .and_then(|(year, month, day)| civil::Date::new(year, month, day).ok())
        .ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Time\\Date::{member} found `{year}-{month}-{day}`, which is not a date"
            ))
        })
}

/// The three civil fields narrowed to the widths [`civil::Date::new`] takes,
/// or `None` for a number no calendar field can hold.
fn parts(year: i64, month: i64, day: i64) -> Option<(i16, i8, i8)> {
    Some((
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    ))
}

/// The date `year-month-day`, or the throw ADR 0063 R4 owes for a triple that
/// is not one — which is the whole of why § 4 has no `checkdate`.
fn date_from_parts(year: i64, month: i64, day: i64, member: &str) -> Result<Value, Fault> {
    let Some((year, month, day)) = parts(year, month, day) else {
        return Err(out_of_range(
            member,
            "a year is within -9999..=9999, and a month and a day are numbers a calendar writes",
        ));
    };
    civil::Date::new(year, month, day)
        .map(date_built)
        .map_err(|err| out_of_range(member, &err.to_string()))
}

/// The `uint` in argument slot `at`, as the `i64` [`parts`] narrows.
///
/// A count past `i64` is handed on as `i64::MAX` rather than reported here, so
/// that "too large for a month" is one message from the calendar rather than
/// two from two range checks.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives.
fn field_at(args: &[Value], at: usize, member: &str, what: &str) -> Result<i64, Fault> {
    args[at]
        .as_uint()
        .map(|held| i64::try_from(held).unwrap_or(i64::MAX))
        .ok_or_else(|| {
            Fault::fatal(format!(
                "{member} expected a `uint` for `{what}`, got tag {}",
                args[at].tag_byte()
            ))
        })
}

/// `count` steps of the unit in argument slot 2, added to the receiver —
/// [`stepped`] one level down, where there is no time of day and no zone.
fn date_stepped(args: &[Value], member: &str, sign: i64) -> Result<Value, Fault> {
    let label = format!(r"Core\Time\Date::{member}");
    let at = date_of(args, 0, member)?;
    let (unit, scale) = unit_of(args, 2, &label)?;
    // § 4's two arithmetics, one level down: a date has no time of day, so a
    // step smaller than a day has nothing here to move. Refusing is the only
    // honest answer — truncating it to zero would make
    // `$d->plus(23, Unit::Hour)` silently the same date, and rounding it to a
    // day would make `plus(1, Unit::Hour)` a day's move.
    if !matches!(
        unit,
        jiff::Unit::Year | jiff::Unit::Month | jiff::Unit::Week | jiff::Unit::Day
    ) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{label}(): a unit smaller than `Unit::Day` does not move a date"),
        ));
    }
    let steps = args[1]
        .as_int()
        .ok_or_else(|| Fault::fatal(format!("{label} expected an `int` count")))?
        .checked_mul(scale)
        .and_then(|steps| steps.checked_mul(sign))
        .ok_or_else(|| {
            out_of_range(
                &label,
                "that many units is past what a calendar span can hold",
            )
        })?;
    let span = span_of(unit, steps).map_err(|err| out_of_range(&label, &err.to_string()))?;
    at.checked_add(span)
        .map(date_built)
        .map_err(|err| out_of_range(&label, &err.to_string()))
}

nvs_runtime::nvs_helper! {
    /// `Date::at(int $year, uint $month, uint $day): Date` — § 4's zone-free
    /// constructor, and the one place a date that does not exist throws.
    fn nvs_core_time_date_at(_ctx, args: [3]) {
        let label = r"Core\Time\Date::at";
        let year = args[0].as_int().ok_or_else(|| {
            Fault::fatal(format!(
                "{label} expected an `int` for `year`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let month = field_at(args, 1, label, "month")?;
        let day = field_at(args, 2, label, "day")?;
        date_from_parts(year, month, day, label)
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->format(string $pattern): string` — [`crate::cldr`]'s patterns
    /// again, narrowed to the letters a date carries.
    ///
    /// A pattern naming an hour or a zone is refused rather than filled in;
    /// [`crate::cldr::date_fields_only`] owns why. What that leaves is a
    /// rendering that reads nothing but the three civil fields the value
    /// holds, which is what makes rendering it without an instant possible at
    /// all.
    fn nvs_core_time_date_format(_ctx, args: [2]) {
        let at = date_of(args, 0, "format")?;
        let pattern = text_of(args, 1, r"Core\Time\Date::format")?;
        let pieces = crate::cldr::compile(pattern)
            .and_then(|pieces| crate::cldr::date_fields_only(&pieces).map(|()| pieces))
            .map_err(|why| Fault::thrown(format!("Core\\Time\\Date::format(): {why}")))?;
        // Rendered from the civil date rather than from a placement on the
        // timeline, because that placement is not total at either end of the
        // year range `Date::at` accepts — [`crate::cldr::render_utc`] owns the
        // reasoning and the two dates that used to abort the request here.
        Ok(Value::str(NvsStr::new(
            crate::cldr::render_utc(&pieces, at.to_datetime(civil::Time::midnight())).as_bytes(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->plus(int $count, Unit $unit): Date` — the same clamping calendar
    /// step [`nvs_core_time_datetime_plus`] takes, so the last day of January
    /// plus a month is the last day of February.
    fn nvs_core_time_date_plus(_ctx, args: [3]) {
        date_stepped(args, "plus", 1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->minus(int $count, Unit $unit): Date`.
    fn nvs_core_time_date_minus(_ctx, args: [3]) {
        date_stepped(args, "minus", -1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->with({year?, month?, day?}): Date` — an omitted option leaves its
    /// field, for [`WITH_OPTIONS`]'s reason.
    ///
    /// A combination that is not a date throws, exactly as the constructor
    /// does: `2024-02-29` with `{year: 2023}` has no answer to clamp to that
    /// is not a guess.
    fn nvs_core_time_date_with(_ctx, args: [4]) {
        let label = r"Core\Time\Date::with";
        let mut building = date_of(args, 0, "with")?.with();
        if let Some(year) = optional(args, 1, label, "year")? {
            building = building.year(i16::try_from(year).map_err(|_| {
                out_of_range(label, "`year` is outside -9999..=9999")
            })?);
        }
        for (index, name) in [(2, "month"), (3, "day")] {
            let Some(value) = optional(args, index, label, name)? else {
                continue;
            };
            let value = i8::try_from(value).map_err(|_| {
                out_of_range(label, &format!("`{name}` is outside what that field can hold"))
            })?;
            building = if name == "month" {
                building.month(value)
            } else {
                building.day(value)
            };
        }
        building
            .build()
            .map(date_built)
            .map_err(|err| out_of_range(label, &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$d->compareTo(Date $other): int` — `Comparable`'s member, over the
    /// one order a civil date has.
    fn nvs_core_time_date_compare_to(_ctx, args: [2]) {
        let left = date_of(args, 0, "compareTo")?;
        let right = date_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

// ============================================================================
// `Core\Time\TimeOfDay` — reading, building and the clock step
// ============================================================================

/// A fresh `TimeOfDay` at `at`.
fn clock_built(at: civil::Time) -> Value {
    crate::instance::build(
        &TIME_OF_DAY,
        [
            Value::int(i64::from(at.hour())),
            Value::int(i64::from(at.minute())),
            Value::int(i64::from(at.second())),
            Value::int(i64::from(at.subsec_nanosecond())),
        ],
    )
}

/// The [`civil::Time`] the `TimeOfDay` in argument slot `at` holds.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for [`date_of`]'s reason.
fn clock_of(args: &[Value], at: usize, member: &str) -> Result<civil::Time, Fault> {
    let object = crate::instance::receiver(args[at], &TIME_OF_DAY, member)?;
    let read = |index: usize, what: &str| {
        crate::instance::slot(object, index)
            .as_int()
            .ok_or_else(|| {
                Fault::fatal(format!(
                    "Core\\Time\\TimeOfDay::{member} found a non-`int` `{what}` slot"
                ))
            })
    };
    let hour = read(CLOCK_HOUR_SLOT, "hour")?;
    let minute = read(CLOCK_MINUTE_SLOT, "minute")?;
    let second = read(CLOCK_SECOND_SLOT, "second")?;
    let nanos = read(CLOCK_NANOS_SLOT, "nanos")?;
    clock_from_parts(hour, minute, second, nanos).ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Time\\TimeOfDay::{member} found `{hour}:{minute}:{second}.{nanos}`, which \
                 is not a time"
        ))
    })
}

/// The four fields as a [`civil::Time`], or `None` for a combination that is
/// not one — including a number no clock field can hold.
fn clock_from_parts(hour: i64, minute: i64, second: i64, nanos: i64) -> Option<civil::Time> {
    let hour = i8::try_from(hour).ok()?;
    let minute = i8::try_from(minute).ok()?;
    let second = i8::try_from(second).ok()?;
    let nanos = i32::try_from(nanos).ok()?;
    civil::Time::new(hour, minute, second, nanos).ok()
}

/// `count` steps of the unit in argument slot 2, added to the receiver —
/// [`date_stepped`]'s opposite number, and the other half of the same split.
///
/// **The step wraps within the day**, so `23:30` plus an hour is `00:30`. A
/// time of day is a position in the 24-hour cycle with no date under it, so
/// there is nowhere for a carry to go; throwing at midnight instead would make
/// `plus` a member whose safety depends on the value it is called on, which is
/// the landmine a total operation avoids. `$d->plus(…)` on a `DateTime` is
/// where a step that carries a day belongs.
///
/// **The wrap is exact at every count the span bound accepts**, which is what
/// [`clock_cycle`] is for; the count is reduced to one turn of the dial before
/// `jiff` ever sees it.
fn clock_stepped(args: &[Value], member: &str, sign: i64) -> Result<Value, Fault> {
    let label = format!(r"Core\Time\TimeOfDay::{member}");
    let at = clock_of(args, 0, member)?;
    let (unit, scale) = unit_of(args, 2, &label)?;
    // A unit of a day or larger has nothing here to move, exactly as a unit
    // smaller than a day has nothing in a `Date` to move.
    if matches!(
        unit,
        jiff::Unit::Year | jiff::Unit::Month | jiff::Unit::Week | jiff::Unit::Day
    ) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!("{label}(): a unit of `Unit::Day` or larger does not move a time of day"),
        ));
    }
    let steps = args[1]
        .as_int()
        .ok_or_else(|| Fault::fatal(format!("{label} expected an `int` count")))?
        .checked_mul(scale)
        .and_then(|steps| steps.checked_mul(sign))
        .ok_or_else(|| out_of_range(&label, "that many units is past what a span can hold"))?;
    // Built from the whole count first, and thrown away: a span's per-unit
    // bound is what refuses a count no span can carry, and its message names
    // the range. It is deliberately not what moves the dial. `jiff` totals a
    // span in `i64` nanoseconds, so a count past roughly 2,562,047 hours wraps
    // *there* — silently, and onto a reading that is not the modular one this
    // member promises. Reducing to one turn of the dial first makes the answer
    // exact at every count the bound accepts, which is the whole accepted
    // range rather than the first thousandth of it.
    span_of(unit, steps).map_err(|err| out_of_range(&label, &err.to_string()))?;
    let span = span_of(unit, steps % clock_cycle(unit))
        .map_err(|err| out_of_range(&label, &err.to_string()))?;
    Ok(clock_built(at.wrapping_add(span)))
}

/// How many steps of `unit` are one whole turn of the dial — a day, written in
/// that unit.
///
/// Every unit that reaches here divides a day exactly, so a count reduced by
/// this lands on the reading the whole count would have landed on if the
/// arithmetic underneath were unbounded. [`clock_stepped`] has already refused
/// every unit of a day or larger, which leaves nanoseconds as the only case the
/// arm below does not name.
fn clock_cycle(unit: jiff::Unit) -> i64 {
    /// One turn of the dial, in nanoseconds.
    const NANOS_PER_DAY: i64 = 86_400_000_000_000;
    NANOS_PER_DAY
        / match unit {
            jiff::Unit::Hour => 3_600_000_000_000,
            jiff::Unit::Minute => 60_000_000_000,
            jiff::Unit::Second => 1_000_000_000,
            jiff::Unit::Millisecond => 1_000_000,
            jiff::Unit::Microsecond => 1_000,
            _ => 1,
        }
}

nvs_runtime::nvs_helper! {
    /// `TimeOfDay::at(uint $hour, uint $minute, {second?, nanos?}):
    /// TimeOfDay` — the zone-free constructor, with the two fields a wall
    /// clock usually leaves off defaulting to zero ([`AT_OPTIONS`]).
    fn nvs_core_time_of_day_at(_ctx, args: [4]) {
        let label = r"Core\Time\TimeOfDay::at";
        let hour = field_at(args, 0, label, "hour")?;
        let minute = field_at(args, 1, label, "minute")?;
        let second = field_at(args, 2, label, "second")?;
        let nanos = field_at(args, 3, label, "nanos")?;
        clock_from_parts(hour, minute, second, nanos)
            .map(clock_built)
            .ok_or_else(|| {
                out_of_range(
                    label,
                    &format!("`{hour}:{minute}:{second}.{nanos}` is not a time of day"),
                )
            })
    }
}

nvs_runtime::nvs_helper! {
    /// `$t->format(string $pattern): string` — [`crate::cldr`]'s patterns
    /// narrowed the other way, to the letters a clock carries
    /// ([`crate::cldr::time_fields_only`]).
    fn nvs_core_time_of_day_format(_ctx, args: [2]) {
        let at = clock_of(args, 0, "format")?;
        let pattern = text_of(args, 1, r"Core\Time\TimeOfDay::format")?;
        let pieces = crate::cldr::compile(pattern)
            .and_then(|pieces| crate::cldr::time_fields_only(&pieces).map(|()| pieces))
            .map_err(|why| Fault::thrown(format!("Core\\Time\\TimeOfDay::format(): {why}")))?;
        // The date below is arbitrary and unobservable: no piece that survived
        // `time_fields_only` reads a calendar field or a zone. Rendered civil
        // rather than placed on the timeline for [`crate::cldr::render_utc`]'s
        // reason, which is the sibling `Core\Time\Date::format`'s and applies
        // here for symmetry.
        Ok(Value::str(NvsStr::new(
            crate::cldr::render_utc(&pieces, civil::Date::constant(1970, 1, 1).to_datetime(at))
                .as_bytes(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `$t->plus(int $count, Unit $unit): TimeOfDay` — see [`clock_stepped`]
    /// for what happens at midnight.
    fn nvs_core_time_of_day_plus(_ctx, args: [3]) {
        clock_stepped(args, "plus", 1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$t->minus(int $count, Unit $unit): TimeOfDay`.
    fn nvs_core_time_of_day_minus(_ctx, args: [3]) {
        clock_stepped(args, "minus", -1)
    }
}

nvs_runtime::nvs_helper! {
    /// `$t->with({hour?, minute?, second?, nanos?}): TimeOfDay` — an omitted
    /// option leaves its field, for [`WITH_OPTIONS`]'s reason.
    fn nvs_core_time_of_day_with(_ctx, args: [5]) {
        let label = r"Core\Time\TimeOfDay::with";
        let mut building = clock_of(args, 0, "with")?.with();
        for (index, name) in [(1, "hour"), (2, "minute"), (3, "second")] {
            let Some(value) = optional(args, index, label, name)? else {
                continue;
            };
            let value = i8::try_from(value).map_err(|_| {
                out_of_range(label, &format!("`{name}` is outside what that field can hold"))
            })?;
            building = match name {
                "hour" => building.hour(value),
                "minute" => building.minute(value),
                _ => building.second(value),
            };
        }
        if let Some(nanos) = optional(args, 4, label, "nanos")? {
            building = building.subsec_nanosecond(i32::try_from(nanos).map_err(|_| {
                out_of_range(label, "`nanos` is a subsecond count")
            })?);
        }
        building
            .build()
            .map(clock_built)
            .map_err(|err| out_of_range(label, &err.to_string()))
    }
}

nvs_runtime::nvs_helper! {
    /// `$t->compareTo(TimeOfDay $other): int` — `Comparable`'s member, over
    /// the one order a clock reading has.
    fn nvs_core_time_of_day_compare_to(_ctx, args: [2]) {
        let left = clock_of(args, 0, "compareTo")?;
        let right = clock_of(args, 1, "compareTo")?;
        Ok(Value::int(match left.cmp(&right) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
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
        assert_eq!(DATETIME_SECONDS_SLOT, DATETIME.slot("seconds"));
        assert_eq!(DATETIME_NANOS_SLOT, DATETIME.slot("nanos"));
        assert_eq!(DATETIME_ZONE_SLOT, DATETIME.slot("zone"));
        assert_eq!(DATETIME.slots.len(), 3);
        assert_eq!(DATE_YEAR_SLOT, DATE.slot("year"));
        assert_eq!(DATE_MONTH_SLOT, DATE.slot("month"));
        assert_eq!(DATE_DAY_SLOT, DATE.slot("day"));
        assert_eq!(DATE.slots.len(), 3);
        assert_eq!(CLOCK_HOUR_SLOT, TIME_OF_DAY.slot("hour"));
        assert_eq!(CLOCK_MINUTE_SLOT, TIME_OF_DAY.slot("minute"));
        assert_eq!(CLOCK_SECOND_SLOT, TIME_OF_DAY.slot("second"));
        assert_eq!(CLOCK_NANOS_SLOT, TIME_OF_DAY.slot("nanos"));
        assert_eq!(TIME_OF_DAY.slots.len(), 4);
    }

    /// Every symbol this module's classes register resolves in its own
    /// `address` — the miss that is a *runtime* panic rather than a link
    /// error, so it is worth a test of its own.
    #[test]
    fn every_registered_symbol_has_an_address_here() {
        for class in [
            &TIME,
            &INSTANT,
            &DATETIME,
            &DATE,
            &TIME_OF_DAY,
            &DURATION,
            &ZONE,
        ] {
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
