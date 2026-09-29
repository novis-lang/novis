//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 4 — `Core\Time` and the types it answers with, one module because they
//! are one domain: `Duration` (the type
//! `rule:types/duration-literal`'s `30s`/`1h30m`
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
//! What `jiff` buys is that its own types
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
//! `nvs.toml` will call for a duration-valued directive. `rule:types/duration-literal` requires
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
//! fields. A duration *literal* spends an allocation too: `rule:types/duration-literal`'s constant-pool folding wants an *immortal* value with no allocation at
//! all, which is the same thing `nvs_runtime`'s own gap 3 owes a string
//! literal, so both close together rather than one growing a mechanism the
//! other does not use. Until then `30s` is one
//! [`nvs_core_time_duration_nanoseconds`] call on a folded constant, which is
//! already the whole grammar resolved at compile time.
//!
//! # There is no `Core\Month`, and there is not going to be one
//!
//! § 4 writes no member that takes or answers with one, `Core\Weekday` existing
//! only because `$d->weekday()` does, and an enum nothing names is surface with
//! no spec home. This is written down because it is the question that keeps
//! being re-asked, not because anything is missing: three earlier notes here
//! described a member the spec never had. **§ 4 itself is whole** — `withTime`,
//! its last row and its only one that *takes* a component view rather than
//! answering with one, is [`nvs_core_time_datetime_with_time`].
//!
//! # `sleep` parks the task, and a wait off a core still blocks its thread
//!
//! `Core\Time::sleep` parks the task rather than blocking the core, and what is
//! left of it is one thing: a wait *off* a core still blocks the thread it is
//! on, which is right for `nvs run` and is what a worker's blocking pool would
//! otherwise be for. The route is [`nvs_runtime::host::Host::sleep`] and the
//! mechanism is `nvs-host`'s `timer` module — this task's own deadline on the
//! reactor the core is about to poll. A cancellation mid-sleep comes back as
//! [`nvs_runtime::host::Woken::Cancelled`] and the member stops the request
//! through [`nvs_runtime::Ctx::cancel`], because a parked task standing on an
//! `extern "C"` frame is resumed rather than unwound; `nvs_host`'s scheduler
//! module doc owns that decision.
//!
//! # `Comparable` and `Stringable` are satisfied by member, not by declaration
//!
//! The member is the whole of it, because a `Core` class writes no `implements`
//! clause anywhere. `Duration`, `Instant`, `Date` and `TimeOfDay` each register
//! `compareTo`, which is what `nvs_stdlib::registry::implements_comparable`
//! reads and `nvs_types::core_lib` seeds
//! `rule:classes/ordering-lowers-to-compare-to`'s conformance from, so
//! `$a < $b` orders two of them and lowers to that same member —
//! `nvs_ir::lower::operator::lower_object_comparison` takes the `Core` branch,
//! since a helper symbol has no entry in any compiled method table.
//! `"took " . $d` is the same story one interface along:
//! `rule:classes/stringable`'s rendering is decided by the registered
//! `toString` rather than by a declaration, through
//! `nvs_types::expr::operators::require_stringable` where the operand's type
//! names this class and `nvs_stdlib::instance`'s descriptor renderer where it
//! names none.
//!
//! # A written pattern is prepared, an assembled one is not
//!
//! `$d->format` and `Core\Time::parse` are `rule:expressions/intrinsic-literals`'s
//! intrinsics: a pattern written as a literal is compiled while checking, so a
//! malformed one is a diagnostic rather than a throw, and the call carries
//! [`crate::cldr::PREPARED_PATTERN`] as
//! `crate::registry::PREPARED_MEMBERS`' argument 0 — which is what lets this
//! core compile that pattern once and keep it. A pattern the program assembled
//! carries [`crate::cldr::PREPARED_NONE`], is compiled at the call and is kept
//! nowhere; [`crate::cldr`]'s own docs own why the second is not cached. The
//! `format` members `Core\Time\Date` and `Core\Time\TimeOfDay` own are off that
//! roster and take the second path always.
//!
//! # What these members do with a qualifier
//!
//! `rule:security/unclassified-parameter-refuses-tainted`'s classification splits this module's `string` parameters in
//! two, and neither half is [`Qual::Contagious`] — which is unusual enough to
//! be worth the paragraph.
//!
//! * **Every pattern is a [`Qual::Sink`].** `rule:core-api/shape-rules` R11's third grammar is
//!   the CLDR date pattern, and `rule:security/sink-predicate`'s corollary makes a grammar a
//!   sink wherever it is declared: the three `format` members' one parameter,
//!   and `Core\Time::parse`'s *second*. A pattern is an instruction to
//!   [`crate::cldr`], so a `tainted` one is refused rather than compiled.
//! * **Every parsed text is [`Qual::Neutral`]** — `Core\Time::fromIso`,
//!   `Core\Time::parse`'s first argument, `Duration::parse` and `Zone::of`.
//!   Each answers a value from a **closed space**: an instant, a civil
//!   date-time, a magnitude of nanoseconds, an entry of the IANA roster. No
//!   byte of the argument survives into any of them, and rendering one back
//!   goes through a pattern the *call site* wrote, so there is nothing for a
//!   qualifier to travel on. This is `rule:security/taint-propagation`'s "a checked conversion
//!   launders" reached at a member rather than at a cast, and it is the same
//!   judgement `rule:routing/a-capture-narrows-to-a-closed-set` makes when it narrows a route capture to a closed
//!   set with a type.
//!
//!   The line it draws is the one `Core\Bytes::at` is on the other side of:
//!   `at` answers a `uint` and is contagious anyway, because that `uint` *is*
//!   a byte of the subject and `fill`/`join` put the buffer back together out
//!   of those numbers. A parse's answer cannot be taken apart into the text it
//!   came from. The neighbouring class to be careful with is `Core\Uri`, whose
//!   `parse` keeps the host and the path as text and so is contagious.

use std::sync::OnceLock;
use std::time::SystemTime;

use jiff::civil::{self, Weekday};
use jiff::fmt::temporal::Pieces;
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp, Zoned};
use nvs_runtime::host::Woken;
use nvs_runtime::{Ctx, Fault, NvsStr, ThrownClass, Value};
use nvs_syntax::duration;

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy,
    EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
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
/// count; a constant one is `rule:types/duration-literal`'s literal, and the two produce the same
/// value through this class's one slot.
pub const DURATION: CoreClass = CoreClass {
    name: DURATION_NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "nanoseconds",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: FROM_NANOS_SYMBOL,
            doc: Some(&DURATION_NANOSECONDS_DOC),
        },
        CoreMethod {
            name: "microseconds",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_microseconds",
            doc: Some(&DURATION_MICROSECONDS_DOC),
        },
        CoreMethod {
            name: "milliseconds",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_milliseconds",
            doc: Some(&DURATION_MILLISECONDS_DOC),
        },
        CoreMethod {
            name: "seconds",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_seconds",
            doc: Some(&DURATION_SECONDS_DOC),
        },
        CoreMethod {
            name: "minutes",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_minutes",
            doc: Some(&DURATION_MINUTES_DOC),
        },
        CoreMethod {
            name: "hours",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_hours",
            doc: Some(&DURATION_HOURS_DOC),
        },
        CoreMethod {
            name: "days",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_days",
            doc: Some(&DURATION_DAYS_DOC),
        },
        CoreMethod {
            name: "weeks",
            names: &["n"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_weeks",
            doc: Some(&DURATION_WEEKS_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_parse",
            doc: Some(&DURATION_PARSE_DOC),
        },
    ],
    instance: &[
        CoreMethod {
            name: "toNanoseconds",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_nanoseconds",
            doc: Some(&DURATION_TO_NANOSECONDS_DOC),
        },
        CoreMethod {
            name: "toMicroseconds",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_microseconds",
            doc: Some(&DURATION_TO_MICROSECONDS_DOC),
        },
        CoreMethod {
            name: "toMilliseconds",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_milliseconds",
            doc: Some(&DURATION_TO_MILLISECONDS_DOC),
        },
        CoreMethod {
            name: "toSeconds",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_to_seconds",
            doc: Some(&DURATION_TO_SECONDS_DOC),
        },
        CoreMethod {
            name: "plus",
            names: &["d"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_plus",
            doc: Some(&DURATION_PLUS_DOC),
        },
        CoreMethod {
            name: "minus",
            names: &["d"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_minus",
            doc: Some(&DURATION_MINUS_DOC),
        },
        CoreMethod {
            name: "multipliedBy",
            names: &["factor"],
            params: &[CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_multiplied_by",
            doc: Some(&DURATION_MULTIPLIED_BY_DOC),
        },
        CoreMethod {
            name: "negated",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_duration_negated",
            doc: Some(&DURATION_NEGATED_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_duration_compare_to",
            doc: Some(&DURATION_COMPARE_TO_DOC),
        },
        CoreMethod {
            name: "toString",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_duration_to_string",
            doc: Some(&DURATION_TO_STRING_DOC),
        },
    ],
    slots: &["nanos"],
    constants: &[],
};

/// `Core\Time\Duration::nanoseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_NANOSECONDS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of exactly `$n` nanoseconds — the computed-count form of the \
            duration literal, and the member a literal such as `30s` itself reaches a value \
            through.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of nanoseconds; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` nanoseconds.",
    errors: &[],
};

/// `Core\Time\Duration::microseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_MICROSECONDS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` microseconds, for a count computed at run time; a \
            constant one is a duration literal.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of microseconds; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 1000 nanoseconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` microseconds is longer than a `Duration` can hold — past ±2⁶³ nanoseconds.",
    }],
};

/// `Core\Time\Duration::milliseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_MILLISECONDS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` milliseconds, for a count computed at run time; a \
            constant one is a duration literal such as `250ms`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of milliseconds; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 1 000 000 nanoseconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` milliseconds is longer than a `Duration` can hold — past ±2⁶³ nanoseconds.",
    }],
};

/// `Core\Time\Duration::seconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_SECONDS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` seconds, for a count computed at run time; a constant \
            one is written as the literal `30s`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of seconds; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` seconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` seconds is longer than a `Duration` can hold — past ±2⁶³ nanoseconds, \
               about 292 years.",
    }],
};

/// `Core\Time\Duration::minutes`'s reference card — `rule:core-api/reference-card`.
const DURATION_MINUTES_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` minutes, for a count computed at run time; a constant \
            one is a duration literal such as `1h30m`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of minutes; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 60 seconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` minutes is longer than a `Duration` can hold — about 292 years.",
    }],
};

/// `Core\Time\Duration::hours`'s reference card — `rule:core-api/reference-card`.
const DURATION_HOURS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` hours — exact hours of 3600 seconds, which is how \
            `Time::now()->plus(72h)` differs from a calendar step of three days. A constant \
            count is the literal `72h`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of hours; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 3600 seconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` hours is longer than a `Duration` can hold — about 292 years.",
    }],
};

/// `Core\Time\Duration::days`'s reference card — `rule:core-api/reference-card`.
const DURATION_DAYS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` days of exactly 24 hours each — never a calendar day, \
            which `DateTime::plus($n, Unit::Day)` is. A constant count is the literal `30d`.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of 24-hour days; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 86 400 seconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` days is longer than a `Duration` can hold — about 292 years.",
    }],
};

/// `Core\Time\Duration::weeks`'s reference card — `rule:core-api/reference-card`.
const DURATION_WEEKS_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Duration` of `$n` weeks of exactly seven 24-hour days each — never a \
            calendar week, which `DateTime::plus($n, Unit::Week)` is.",
    params: &[ParamDoc {
        name: "n",
        desc: "The count of 168-hour weeks; negative for a duration that runs backwards.",
        shape: &[],
    }],
    ret: "A `Duration` of `$n` × 604 800 seconds.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$n` weeks is longer than a `Duration` can hold — about 292 years.",
    }],
};

/// `Core\Time\Duration::parse`'s reference card — `rule:core-api/reference-card`.
const DURATION_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads the duration literal grammar — `30s`, `1h30m`, `7d` — at run time, \
            through the one implementation the lexer uses for the source literal: the typed \
            form of `strtotime` for an exact offset arriving in a config value or a flag. It \
            accepts nothing else, so a `tainted` value comes out laundered.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to read, in the duration literal grammar.",
        shape: &[],
    }],
    ret: "The `Duration` the text spells.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$text` is not a duration literal — an unknown or repeated unit, a missing \
               count, trailing text — or spells more than a `Duration` can hold. A `$text` \
               written as a literal is read by this same grammar while checking and refused \
               there as `E0769`, so only a computed one reaches this throw.",
    }],
};

/// `Core\Time\Duration::toNanoseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_TO_NANOSECONDS_DOC: MethodDoc = MethodDoc {
    short: "Answers the receiver as a count of nanoseconds, which is exactly what it holds.",
    params: &[],
    ret: "The whole count of nanoseconds; negative for a duration that runs backwards.",
    errors: &[],
};

/// `Core\Time\Duration::toMicroseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_TO_MICROSECONDS_DOC: MethodDoc = MethodDoc {
    short: "Answers the receiver as a count of whole microseconds.",
    params: &[],
    ret: "The nanosecond count divided by 1000, truncated toward zero.",
    errors: &[],
};

/// `Core\Time\Duration::toMilliseconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_TO_MILLISECONDS_DOC: MethodDoc = MethodDoc {
    short: "Answers the receiver as a count of whole milliseconds.",
    params: &[],
    ret: "The nanosecond count divided by 1 000 000, truncated toward zero.",
    errors: &[],
};

/// `Core\Time\Duration::toSeconds`'s reference card — `rule:core-api/reference-card`.
const DURATION_TO_SECONDS_DOC: MethodDoc = MethodDoc {
    short: "Answers the receiver as a count of whole seconds.",
    params: &[],
    ret: "The nanosecond count divided by 1 000 000 000, truncated toward zero — `0` for \
          anything shorter than a second.",
    errors: &[],
};

/// `Core\Time\Duration::plus`'s reference card — `rule:core-api/reference-card`.
const DURATION_PLUS_DOC: MethodDoc = MethodDoc {
    short: "Adds `$d` to the receiver, nanosecond for nanosecond.",
    params: &[ParamDoc {
        name: "d",
        desc: "The duration to add; a negative one shortens the result.",
        shape: &[],
    }],
    ret: "A new `Duration` of the sum; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The sum is longer than a `Duration` can hold.",
    }],
};

/// `Core\Time\Duration::minus`'s reference card — `rule:core-api/reference-card`.
const DURATION_MINUS_DOC: MethodDoc = MethodDoc {
    short: "Subtracts `$d` from the receiver, nanosecond for nanosecond.",
    params: &[ParamDoc {
        name: "d",
        desc: "The duration to subtract; a negative one lengthens the result.",
        shape: &[],
    }],
    ret: "A new `Duration` of the difference, negative where `$d` is the longer; the receiver \
          is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The difference is longer than a `Duration` can hold.",
    }],
};

/// `Core\Time\Duration::multipliedBy`'s reference card — `rule:core-api/reference-card`.
const DURATION_MULTIPLIED_BY_DOC: MethodDoc = MethodDoc {
    short: "Scales the receiver by a whole factor.",
    params: &[ParamDoc {
        name: "factor",
        desc: "The integer to multiply by; `0` answers an empty duration and a negative factor \
               reverses the direction.",
        shape: &[],
    }],
    ret: "A new `Duration` of `$factor` times the receiver; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The product is longer than a `Duration` can hold.",
    }],
};

/// `Core\Time\Duration::negated`'s reference card — `rule:core-api/reference-card`.
const DURATION_NEGATED_DOC: MethodDoc = MethodDoc {
    short: "Reverses the receiver's direction: `30s` becomes `-30s`.",
    params: &[],
    ret: "A new `Duration` of the same length in the opposite direction; the receiver is \
          unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The receiver is the one duration whose negation does not fit — exactly −2⁶³ \
               nanoseconds.",
    }],
};

/// `Core\Time\Duration::compareTo`'s reference card — `rule:core-api/reference-card`.
const DURATION_COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Orders two durations by length and sign, as `Comparable` requires, so a \
            shorter duration compares below a longer one and a negative one below every \
            positive one. This is also how two durations are compared by content: `==` on \
            two objects is identity, so `90m == 1h30m` is `false` and \
            `$a->compareTo($b) == 0` is the question it looks like it asks.",
    params: &[ParamDoc {
        name: "other",
        desc: "The duration to compare against.",
        shape: &[],
    }],
    ret: "`-1` when the receiver is shorter than `$other`, `0` when they are equal, `1` when \
          it is longer.",
    errors: &[],
};

/// `Core\Time\Duration::toString`'s reference card — `rule:core-api/reference-card`.
const DURATION_TO_STRING_DOC: MethodDoc = MethodDoc {
    short: "Renders the receiver in the duration literal grammar — `1h30m`, `250ms` — as \
            `Stringable` requires, so the text round-trips through `Duration::parse`.",
    params: &[],
    ret: "The literal spelling of the receiver.",
    errors: &[],
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

/// A `Core\Time\Duration` of `nanos`, for another module answering one.
///
/// The out-of-crate-module half of [`built`], and the counterpart to
/// [`nanos_of`] — spec § 4's type belongs to this module, so a member elsewhere
/// that answers a `Duration` builds it through here rather than reaching for
/// [`DURATION`]'s layout. `Core\RateLimit\Decision`'s `retryAfter` is the first.
pub(crate) fn duration_of(nanos: i64) -> Value {
    built(nanos)
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

/// `rule:core-api/shape-rules` R4: a `Duration` past `i64` nanoseconds throws rather than wraps,
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
    /// Also where a **literal** lands: `rule:types/duration-literal` folds `1h30m` to its
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
    /// day, never a calendar day (`rule:types/duration-literal`). A calendar step is
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
    /// `Core\Time\Duration::parse(string $text): Duration` — `rule:types/duration-literal`'s
    /// run-time entry point into the *same* grammar the lexer reads, so
    /// `Duration::parse("1h30m")` and the literal `1h30m` are one value.
    ///
    /// Throws on anything it does not accept, which is what makes it an
    /// `rule:security/tainted-qualifier`
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
    /// `$d->plus(Duration $other): Duration` — a fresh value, since `rule:core-api/shape-rules`
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
    /// `$d->minus(Duration $other): Duration` — the spelling `rule:types/duration-literal`
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
    /// (`rule:classes/comparable`),
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
    /// (`rule:classes/no-magic-methods`),
    /// emitting `rule:types/duration-literal`'s grammar so that a value round-trips through
    /// `parse` — over the durations that grammar can spell, which is the
    /// non-negative ones. This member is total and a negative duration is
    /// reachable through `minus` and `negated`, so it renders one with a
    /// leading `-` that `rule:types/duration-literal` has `parse` refuse by name.
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
    doc: None,
    methods: &[
        CoreMethod {
            name: "of",
            names: &["id"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_of",
            doc: Some(&ZONE_OF_DOC),
        },
        CoreMethod {
            name: "fixed",
            names: &["offset"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_fixed",
            doc: Some(&ZONE_FIXED_DOC),
        },
        CoreMethod {
            name: "system",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_zone_system",
            doc: Some(&ZONE_SYSTEM_DOC),
        },
    ],
    instance: &[CoreMethod {
        name: "offsetAt",
        names: &["i"],
        params: &[CoreTy::Instance(INSTANT_NAME)],
        defaults: &[],
        return_ty: CoreTy::Instance(DURATION_NAME),
        symbol: "nvs_core_time_zone_offset_at",
        doc: Some(&ZONE_OFFSET_AT_DOC),
    }],
    slots: &["id"],
    constants: &[CoreConst {
        name: "UTC",
        ty: CoreTy::Instance(ZONE_NAME),
        value: Const::Built {
            symbol: "nvs_core_time_zone_of",
            args: &[Const::Str("UTC")],
        },
        desc: "The UTC zone — the only zone that is ever a default, and only where a call \
               site writes it explicitly.",
    }],
};

/// `Core\Time\Zone::of`'s reference card — `rule:core-api/reference-card`.
const ZONE_OF_DOC: MethodDoc = MethodDoc {
    short: "Looks an IANA identifier such as `Europe/Berlin` up in the bundled time-zone \
            database, replacing `new DateTimeZone(...)` — and throws on one it does not have \
            rather than falling back to UTC.",
    params: &[ParamDoc {
        name: "id",
        desc: "An IANA zone identifier; a `+02:00` offset spelling is `Zone::fixed`'s and is \
               refused here.",
        shape: &[],
    }],
    ret: "The `Zone` the identifier names.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$id` is not in the IANA database, or begins with a sign.",
    }],
};

/// `Core\Time\Zone::fixed`'s reference card — `rule:core-api/reference-card`.
const ZONE_FIXED_DOC: MethodDoc = MethodDoc {
    short: "Builds a zone at a fixed offset from UTC, with no DST rules — for a timestamp that \
            carries an offset rather than a region, which is every RFC 3339 string.",
    params: &[ParamDoc {
        name: "offset",
        desc: "The offset east of UTC, a whole number of seconds; negative for a zone west of \
               Greenwich.",
        shape: &[],
    }],
    ret: "A `Zone` whose identifier is the offset's `±HH:MM[:SS]` spelling.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$offset` has a subsecond part, or lies outside ±25:59:59 of UTC.",
    }],
};

/// `Core\Time\Zone::system`'s reference card — `rule:core-api/reference-card`.
const ZONE_SYSTEM_DOC: MethodDoc = MethodDoc {
    short: "Answers the host's configured zone, replacing `date_default_timezone_get` — as an \
            ordinary value a program passes on explicitly, never an ambient default; there is \
            no `date_default_timezone_set`.",
    params: &[],
    ret: "The host's `Zone` under its IANA name, or as a fixed offset where the host names \
          none (a bare `TZ=+02:00`, an unmapped Windows zone).",
    errors: &[],
};

/// `Core\Time\Zone::offsetAt`'s reference card — `rule:core-api/reference-card`.
const ZONE_OFFSET_AT_DOC: MethodDoc = MethodDoc {
    short: "Answers the zone's offset from UTC at a given instant, replacing `getOffset` — an \
            instant because a zone with DST has no single offset: `Europe/Berlin` is `+01:00` \
            in January and `+02:00` in July.",
    params: &[ParamDoc {
        name: "i",
        desc: "The instant to read the offset at.",
        shape: &[],
    }],
    ret: "The offset east of UTC as a `Duration` of whole seconds; negative west of \
          Greenwich, zero for UTC.",
    errors: &[],
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
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "in",
            names: &["zone"],
            params: &[CoreTy::Instance(ZONE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_instant_in",
            doc: Some(&INSTANT_IN_DOC),
        },
        CoreMethod {
            name: "toEpochSeconds",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_seconds",
            doc: Some(&INSTANT_TO_EPOCH_SECONDS_DOC),
        },
        CoreMethod {
            name: "toEpochMillis",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_millis",
            doc: Some(&INSTANT_TO_EPOCH_MILLIS_DOC),
        },
        CoreMethod {
            name: "toEpochMicros",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_to_epoch_micros",
            doc: Some(&INSTANT_TO_EPOCH_MICROS_DOC),
        },
        CoreMethod {
            name: "plus",
            names: &["d"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_instant_plus",
            doc: Some(&INSTANT_PLUS_DOC),
        },
        CoreMethod {
            name: "minus",
            names: &["d"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_instant_minus",
            doc: Some(&INSTANT_MINUS_DOC),
        },
        CoreMethod {
            name: "since",
            names: &["earlier"],
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_instant_since",
            doc: Some(&INSTANT_SINCE_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(INSTANT_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_instant_compare_to",
            doc: Some(&INSTANT_COMPARE_TO_DOC),
        },
        CoreMethod {
            name: "toIso",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_instant_to_iso",
            doc: Some(&INSTANT_TO_ISO_DOC),
        },
    ],
    slots: &["seconds", "nanos"],
    constants: &[],
};

/// `Core\Time\Instant::in`'s reference card — `rule:core-api/reference-card`.
const INSTANT_IN_DOC: MethodDoc = MethodDoc {
    short: "Reads this instant on `$zone`'s calendar — the only instant→calendar conversion \
            there is, which is why no zone is ever implicit.",
    params: &[ParamDoc {
        name: "zone",
        desc: "The zone whose civil date and time to read.",
        shape: &[],
    }],
    ret: "A `DateTime` at this same instant, in `$zone`.",
    errors: &[],
};

/// `Core\Time\Instant::toEpochSeconds`'s reference card — `rule:core-api/reference-card`.
const INSTANT_TO_EPOCH_SECONDS_DOC: MethodDoc = MethodDoc {
    short: "Answers the Unix timestamp, replacing `getTimestamp` and `date(\"U\")`.",
    params: &[],
    ret: "Whole seconds since `1970-01-01T00:00:00Z`, negative before it, with the subsecond \
          part dropped.",
    errors: &[],
};

/// `Core\Time\Instant::toEpochMillis`'s reference card — `rule:core-api/reference-card`.
const INSTANT_TO_EPOCH_MILLIS_DOC: MethodDoc = MethodDoc {
    short: "Answers the Unix timestamp in milliseconds — one of `microtime(true)`'s two \
            halves, as an exact integer rather than a `float`.",
    params: &[],
    ret: "Whole milliseconds since the Unix epoch, truncated toward zero.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The instant is further from the epoch than a 64-bit millisecond count reaches.",
    }],
};

/// `Core\Time\Instant::toEpochMicros`'s reference card — `rule:core-api/reference-card`.
const INSTANT_TO_EPOCH_MICROS_DOC: MethodDoc = MethodDoc {
    short: "Answers the Unix timestamp in microseconds — `microtime`'s other half, as an \
            exact integer.",
    params: &[],
    ret: "Whole microseconds since the Unix epoch, truncated toward zero.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The instant is further from the epoch than a 64-bit microsecond count reaches.",
    }],
};

/// `Core\Time\Instant::plus`'s reference card — `rule:core-api/reference-card`.
const INSTANT_PLUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the instant forward by an exact `Duration`, replacing `date_add` and \
            `modify` for an exact offset — so it crosses a DST boundary without noticing one; \
            a calendar step is `$i->in($zone)->plus($n, Unit::Day)`.",
    params: &[ParamDoc {
        name: "d",
        desc: "The exact duration to add; a negative one moves the instant back.",
        shape: &[],
    }],
    ret: "A new `Instant`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range, about ±9999 years.",
    }],
};

/// `Core\Time\Instant::minus`'s reference card — `rule:core-api/reference-card`.
const INSTANT_MINUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the instant back by an exact `Duration`, replacing `date_sub` for an exact \
            offset.",
    params: &[ParamDoc {
        name: "d",
        desc: "The exact duration to subtract; a negative one moves the instant forward.",
        shape: &[],
    }],
    ret: "A new `Instant`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range, about ±9999 years.",
    }],
};

/// `Core\Time\Instant::since`'s reference card — `rule:core-api/reference-card`.
const INSTANT_SINCE_DOC: MethodDoc = MethodDoc {
    short: "Measures the exact time from `$earlier` to this instant, replacing `date_diff` and \
            `DateInterval` arithmetic with none of that type's \"1 month\" ambiguity.",
    params: &[ParamDoc {
        name: "earlier",
        desc: "The instant to measure from.",
        shape: &[],
    }],
    ret: "The `Duration` from `$earlier` to the receiver — negative when `$earlier` is in \
          fact later, so it is `plus`'s inverse rather than an absolute distance.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The two instants are further apart than a `Duration` can hold, about 292 years.",
    }],
};

/// `Core\Time\Instant::compareTo`'s reference card — `rule:core-api/reference-card`.
const INSTANT_COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Orders two instants on the timeline, as `Comparable` requires.",
    params: &[ParamDoc {
        name: "other",
        desc: "The instant to compare against.",
        shape: &[],
    }],
    ret: "`-1` when the receiver is earlier than `$other`, `0` when they are the same instant, \
          `1` when it is later.",
    errors: &[],
};

/// `Core\Time\Instant::toIso`'s reference card — `rule:core-api/reference-card`.
const INSTANT_TO_ISO_DOC: MethodDoc = MethodDoc {
    short: "Renders the instant as an RFC 3339 timestamp in UTC, replacing `date(DATE_ATOM)` — \
            the one rendering that needs no zone.",
    params: &[],
    ret: "Text such as `2024-03-01T12:00:00Z`, with the fractional seconds included when they \
          are not zero.",
    errors: &[],
};

/// [`INSTANT`]'s slots, by index.
pub(crate) const INSTANT_SECONDS_SLOT: usize = 0;
/// See [`INSTANT_SECONDS_SLOT`].
pub(crate) const INSTANT_NANOS_SLOT: usize = 1;

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
/// [`crate::registry::ENUMS`] states the rule. The values are the CLDR-free ascending order the spec
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
    doc: Some(&UNIT_DOC),
};

/// [`UNIT`]'s reference card — `rule:core-api/reference-card`.
const UNIT_DOC: EnumDoc = EnumDoc {
    short: "The calendar step a `DateTime`, `Date` or `TimeOfDay` moves by, and the unit \
            `startOf`, `endOf` and `difference` count in — the half of § 4's two arithmetics \
            a DST boundary or a short month can lengthen or shorten, listed smallest first.",
    cases: &[
        CaseDoc {
            name: "Nanosecond",
            desc: "One nanosecond, the finest step any value here resolves.",
        },
        CaseDoc {
            name: "Microsecond",
            desc: "One microsecond — 1000 nanoseconds.",
        },
        CaseDoc {
            name: "Millisecond",
            desc: "One millisecond — 1 000 000 nanoseconds.",
        },
        CaseDoc {
            name: "Second",
            desc: "One second of the civil clock.",
        },
        CaseDoc {
            name: "Minute",
            desc: "One minute of the civil clock.",
        },
        CaseDoc {
            name: "Hour",
            desc: "One hour of the civil clock.",
        },
        CaseDoc {
            name: "Day",
            desc: "One calendar day — 23 or 25 hours where the zone crosses a DST boundary.",
        },
        CaseDoc {
            name: "Week",
            desc: "Seven calendar days; `startOf(Unit::Week)` is the Monday, ISO 8601's first day.",
        },
        CaseDoc {
            name: "Month",
            desc: "One calendar month, with the day-of-month clamped to the target month's \
                   length.",
        },
        CaseDoc {
            name: "Quarter",
            desc: "Three calendar months, so `plus(1, Unit::Quarter)` is `plus(3, Unit::Month)` \
                   and `startOf` lands on January, April, July or October.",
        },
        CaseDoc {
            name: "Year",
            desc: "One calendar year, with 29 February clamped to the 28th where the target has \
                   none.",
        },
    ],
};

/// Spec § 4's `Core\Weekday`, Monday-first — ISO-8601's own order, which is
/// what `date("N")` already answers and what [`crate::cldr`]'s tables index
/// by.
///
/// Zero-based rather than `date("N")`'s one-based count because `rule:enums/closed-integer-type` makes
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
    doc: Some(&WEEKDAY_DOC),
};

/// [`WEEKDAY`]'s reference card — `rule:core-api/reference-card`.
const WEEKDAY_DOC: EnumDoc = EnumDoc {
    short: "A day of the week, Monday first as ISO 8601 and `date(\"N\")` order them — what \
            `$d->weekday()` answers and `next`/`previous` take. The order is theirs and the \
            numbering is not: the cases are zero-based, `Monday as int` is `0` and `Sunday` \
            is `6`, where `date(\"N\")` numbers that same order `1` through `7`.",
    cases: &[
        CaseDoc {
            name: "Monday",
            desc: "The first day of the ISO week.",
        },
        CaseDoc {
            name: "Tuesday",
            desc: "The second day of the ISO week.",
        },
        CaseDoc {
            name: "Wednesday",
            desc: "The third day of the ISO week.",
        },
        CaseDoc {
            name: "Thursday",
            desc: "The fourth day of the ISO week.",
        },
        CaseDoc {
            name: "Friday",
            desc: "The fifth day of the ISO week.",
        },
        CaseDoc {
            name: "Saturday",
            desc: "The sixth day of the ISO week.",
        },
        CaseDoc {
            name: "Sunday",
            desc: "The seventh and last day of the ISO week.",
        },
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
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "format",
            names: &["pattern"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_datetime_format",
            doc: Some(&DATETIME_FORMAT_DOC),
        },
        CoreMethod {
            name: "plus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_plus",
            doc: Some(&DATETIME_PLUS_DOC),
        },
        CoreMethod {
            name: "minus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_minus",
            doc: Some(&DATETIME_MINUS_DOC),
        },
        CoreMethod {
            name: "next",
            names: &["w"],
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_next",
            doc: Some(&DATETIME_NEXT_DOC),
        },
        CoreMethod {
            name: "previous",
            names: &["w"],
            params: &[CoreTy::Enum(WEEKDAY.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_previous",
            doc: Some(&DATETIME_PREVIOUS_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_with",
            doc: Some(&DATETIME_WITH_DOC),
        },
        CoreMethod {
            name: "withTime",
            names: &["t"],
            params: &[CoreTy::Instance(TIME_OF_DAY_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_with_time",
            doc: Some(&DATETIME_WITH_TIME_DOC),
        },
        CoreMethod {
            name: "startOf",
            names: &["u"],
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_start_of",
            doc: Some(&DATETIME_START_OF_DOC),
        },
        CoreMethod {
            name: "endOf",
            names: &["u"],
            params: &[CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_datetime_end_of",
            doc: Some(&DATETIME_END_OF_DOC),
        },
        CoreMethod {
            name: "difference",
            names: &["other", "unit"],
            params: &[CoreTy::Instance(DATETIME_NAME), CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_datetime_difference",
            doc: Some(&DATETIME_DIFFERENCE_DOC),
        },
        CoreMethod {
            name: "toInstant",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_datetime_to_instant",
            doc: Some(&DATETIME_TO_INSTANT_DOC),
        },
        CoreMethod {
            name: "date",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_datetime_date",
            doc: Some(&DATETIME_DATE_DOC),
        },
        CoreMethod {
            name: "timeOfDay",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_datetime_time_of_day",
            doc: Some(&DATETIME_TIME_OF_DAY_DOC),
        },
        CoreMethod {
            name: "zone",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(ZONE_NAME),
            symbol: "nvs_core_time_datetime_zone",
            doc: Some(&DATETIME_ZONE_DOC),
        },
        CoreMethod {
            name: "weekday",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(WEEKDAY.name),
            symbol: "nvs_core_time_datetime_weekday",
            doc: Some(&DATETIME_WEEKDAY_DOC),
        },
        CoreMethod {
            name: "dayOfYear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_time_datetime_day_of_year",
            doc: Some(&DATETIME_DAY_OF_YEAR_DOC),
        },
        CoreMethod {
            name: "isLeapYear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_time_datetime_is_leap_year",
            doc: Some(&DATETIME_IS_LEAP_YEAR_DOC),
        },
    ],
    slots: &["seconds", "nanos", "zone"],
    constants: &[],
};

/// `Core\Time\DateTime::format`'s reference card — `rule:core-api/reference-card`.
const DATETIME_FORMAT_DOC: MethodDoc = MethodDoc {
    short: "Renders the civil date and time through a CLDR pattern — `yyyy-MM-dd HH:mm:ss`, \
            `EEEE, d MMMM yyyy` — replacing `date`, `gmdate`, `idate`, `strftime` and \
            `date_format`; names render in CLDR's root locale, since there is no `setlocale`.",
    params: &[ParamDoc {
        name: "pattern",
        desc: "A CLDR pattern in the subset `crates/nvs-stdlib/src/cldr.rs` implements — a \
               grammar, so a `tainted` one is refused before it runs.",
        shape: &[],
    }],
    ret: "The rendered text.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$pattern` does not compile — a field letter outside the implemented subset, or \
               an unterminated quote. A `$pattern` written as a literal is read while checking \
               and refused there as `E0769`, so only a computed one reaches this throw; \
               `Date::format` and `TimeOfDay::format` read the same patterns and are not \
               checked that way, so a bad literal throws there.",
    }],
};

/// `Core\Time\DateTime::plus`'s reference card — `rule:core-api/reference-card`.
const DATETIME_PLUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the value forward by `$count` calendar steps of `$unit`, replacing `date_add`, \
            `modify` and `strtotime`'s relative half: `1, Unit::Month` lands on the same \
            day-of-month clamped to the month's length, and a day across a DST boundary is 23 \
            or 25 hours. The exact half is `$d->toInstant()->plus(24h)`.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves back.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "The calendar unit of each step.",
            shape: &[],
        },
    ],
    ret: "A new `DateTime` in the same zone; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is past what a calendar span can hold, or the result lies outside the \
               representable range, about ±9999 years.",
    }],
};

/// `Core\Time\DateTime::minus`'s reference card — `rule:core-api/reference-card`.
const DATETIME_MINUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the value back by `$count` calendar steps of `$unit`, replacing `date_sub` — \
            `plus` with the count negated, so the two agree at a month end and a DST boundary.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves forward.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "The calendar unit of each step.",
            shape: &[],
        },
    ],
    ret: "A new `DateTime` in the same zone; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is past what a calendar span can hold, or the result lies outside the \
               representable range, about ±9999 years.",
    }],
};

/// `Core\Time\DateTime::next`'s reference card — `rule:core-api/reference-card`.
const DATETIME_NEXT_DOC: MethodDoc = MethodDoc {
    short: "Finds the nearest strictly later day that falls on `$w`, with the time of day \
            preserved — `strtotime(\"next monday\")` as a typed call.",
    params: &[ParamDoc {
        name: "w",
        desc: "The weekday to move to.",
        shape: &[],
    }],
    ret: "A new `DateTime` one to seven days later — a full week when the receiver already \
          falls on `$w`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range.",
    }],
};

/// `Core\Time\DateTime::previous`'s reference card — `rule:core-api/reference-card`.
const DATETIME_PREVIOUS_DOC: MethodDoc = MethodDoc {
    short: "Finds the nearest strictly earlier day that falls on `$w`, with the time of day \
            preserved — `strtotime(\"last monday\")` as a typed call.",
    params: &[ParamDoc {
        name: "w",
        desc: "The weekday to move to.",
        shape: &[],
    }],
    ret: "A new `DateTime` one to seven days earlier — a full week when the receiver already \
          falls on `$w`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range.",
    }],
};

/// `Core\Time\DateTime::with`'s reference card — `rule:core-api/reference-card`.
const DATETIME_WITH_DOC: MethodDoc = MethodDoc {
    short: "Replaces any of the seven civil fields and leaves the rest, replacing `setDate`, \
            `setTime` and `setISODate`; a civil time the zone skips resolves forward past the \
            gap.",
    params: &[
        ParamDoc {
            name: "year",
            desc: "The new year, within `-9999..=9999`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "month",
            desc: "The new month, `1` to `12`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "day",
            desc: "The new day of the month; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "hour",
            desc: "The new hour, `0` to `23`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "minute",
            desc: "The new minute, `0` to `59`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "second",
            desc: "The new second, `0` to `59`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "nanos",
            desc: "The new subsecond nanoseconds, below `1000000000`; omitted leaves the field \
                   alone.",
            shape: &[],
        },
    ],
    ret: "A new `DateTime` in the same zone; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "An option is outside its field's range, or the fields together are not a date \
               that exists — `{day: 31}` on a June value.",
    }],
};

/// `Core\Time\DateTime::withTime`'s reference card — `rule:core-api/reference-card`.
const DATETIME_WITH_TIME_DOC: MethodDoc = MethodDoc {
    short: "Replaces all four clock fields from a `TimeOfDay` and keeps the date and the zone — \
            the common half of `with`, spelled as the operation it is; a civil time the zone \
            skips resolves forward past the gap.",
    params: &[ParamDoc {
        name: "t",
        desc: "The time of day to set, hour through nanosecond.",
        shape: &[],
    }],
    ret: "A new `DateTime` on the same date in the same zone; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range.",
    }],
};

/// `Core\Time\DateTime::startOf`'s reference card — `rule:core-api/reference-card`.
const DATETIME_START_OF_DOC: MethodDoc = MethodDoc {
    short: "Truncates the value down to the first instant of the enclosing `$u` — \
            `startOf(Unit::Day)` is the local midnight, `strtotime(\"today\")`, and \
            `startOf(Unit::Month)` its first day — computed on the civil fields and then placed \
            back in the zone.",
    params: &[ParamDoc {
        name: "u",
        desc: "The unit to truncate to; `Unit::Week` starts on Monday and `Unit::Quarter` on \
               its first month.",
        shape: &[],
    }],
    ret: "A new `DateTime` in the same zone, at or before the receiver.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range.",
    }],
};

/// `Core\Time\DateTime::endOf`'s reference card — `rule:core-api/reference-card`.
const DATETIME_END_OF_DOC: MethodDoc = MethodDoc {
    short: "Moves the value to the last instant of the enclosing `$u`, one nanosecond before \
            the next one starts — `endOf(Unit::Month)` is `strtotime(\"last day of this \
            month\")`; a distinct operation from `startOf` on a day a DST boundary lengthens \
            or shortens.",
    params: &[ParamDoc {
        name: "u",
        desc: "The unit whose last instant to find.",
        shape: &[],
    }],
    ret: "A new `DateTime` in the same zone, at or after the receiver.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The result lies outside the representable range.",
    }],
};

/// `Core\Time\DateTime::difference`'s reference card — `rule:core-api/reference-card`.
const DATETIME_DIFFERENCE_DOC: MethodDoc = MethodDoc {
    short: "Counts whole units of `$unit` from the receiver to `$other`, in the receiver's \
            zone — an age in years, a term in months — replacing `date_diff` and \
            `DateInterval`'s `y`/`m`/`d` fields.",
    params: &[
        ParamDoc {
            name: "other",
            desc: "The value to count to; it is read in the receiver's zone, so the two may \
                   differ in zone.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "The calendar unit to count in.",
            shape: &[],
        },
    ],
    ret: "The whole count, truncated toward zero — negative when `$other` is earlier, `0` \
          within one unit.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The span between the two lies outside what a calendar span can hold.",
    }],
};

/// `Core\Time\DateTime::toInstant`'s reference card — `rule:core-api/reference-card`.
const DATETIME_TO_INSTANT_DOC: MethodDoc = MethodDoc {
    short: "Answers the absolute point on the timeline this civil value names — free, since a \
            `DateTime` already holds one.",
    params: &[],
    ret: "The `Instant`, with the zone dropped.",
    errors: &[],
};

/// `Core\Time\DateTime::date`'s reference card — `rule:core-api/reference-card`.
const DATETIME_DATE_DOC: MethodDoc = MethodDoc {
    short: "Answers the civil date this value reads as where it is, dropping the time and the \
            zone — so a conversion first, `$d->toInstant()->in($z)->date()`, can answer a \
            different day.",
    params: &[],
    ret: "The `Date` component.",
    errors: &[],
};

/// `Core\Time\DateTime::timeOfDay`'s reference card — `rule:core-api/reference-card`.
const DATETIME_TIME_OF_DAY_DOC: MethodDoc = MethodDoc {
    short: "Answers the wall-clock reading, dropping the date and the zone — two zones can \
            read `09:00` at once, which is why it carries neither.",
    params: &[],
    ret: "The `TimeOfDay` component, hour through nanosecond.",
    errors: &[],
};

/// `Core\Time\DateTime::zone`'s reference card — `rule:core-api/reference-card`.
const DATETIME_ZONE_DOC: MethodDoc = MethodDoc {
    short: "Answers the zone this civil value is in — always one the program named, since \
            none is ever ambient.",
    params: &[],
    ret: "The `Zone` component.",
    errors: &[],
};

/// `Core\Time\DateTime::weekday`'s reference card — `rule:core-api/reference-card`.
const DATETIME_WEEKDAY_DOC: MethodDoc = MethodDoc {
    short: "Answers the day of the week, replacing `date(\"N\")` — as the enum case rather \
            than the number.",
    params: &[],
    ret: "The `Weekday` case, `Weekday::Monday` through `Weekday::Sunday`.",
    errors: &[],
};

/// `Core\Time\DateTime::dayOfYear`'s reference card — `rule:core-api/reference-card`.
const DATETIME_DAY_OF_YEAR_DOC: MethodDoc = MethodDoc {
    short: "Answers the ordinal day within the year, replacing `date(\"z\")`.",
    params: &[],
    ret: "`1` for 1 January through `365` or `366` — one-based where `date(\"z\")` is \
          zero-based, as every other day count here is.",
    errors: &[],
};

/// `Core\Time\DateTime::isLeapYear`'s reference card — `rule:core-api/reference-card`.
const DATETIME_IS_LEAP_YEAR_DOC: MethodDoc = MethodDoc {
    short: "Answers whether the value's year has a 29 February, replacing `date(\"L\")` and \
            `checkdate`'s year half; the day half has no equivalent because a date that does \
            not exist throws where it is built.",
    params: &[],
    ret: "`true` in a leap year of the proleptic Gregorian calendar.",
    errors: &[],
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
    doc: Some(&TIME_CARD),
    methods: &[
        CoreMethod {
            name: "now",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_now",
            doc: Some(&TIME_NOW_DOC),
        },
        CoreMethod {
            name: "monotonic",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(DURATION_NAME),
            symbol: "nvs_core_time_monotonic",
            doc: Some(&TIME_MONOTONIC_DOC),
        },
        CoreMethod {
            name: "sleep",
            names: &["d"],
            params: &[CoreTy::Instance(DURATION_NAME)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_time_sleep",
            doc: Some(&TIME_SLEEP_DOC),
        },
        CoreMethod {
            name: "fromEpoch",
            names: &["seconds"],
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
            doc: Some(&TIME_FROM_EPOCH_DOC),
        },
        CoreMethod {
            name: "fromIso",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(INSTANT_NAME),
            symbol: "nvs_core_time_from_iso",
            doc: Some(&TIME_FROM_ISO_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["text", "format", "zone"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Text(Qual::Sink),
                CoreTy::Instance(ZONE_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(DATETIME_NAME),
            symbol: "nvs_core_time_parse",
            doc: Some(&TIME_PARSE_DOC),
        },
        CoreMethod {
            name: "at",
            names: &["year", "month", "day", "zone"],
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
            doc: Some(&TIME_AT_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Time`'s class card — `rule:core-api/reference-card`.
const TIME_CARD: ClassDoc = ClassDoc {
    short: "Reads the clock and builds dates and times. `now` returns the current `Instant`. \
            `at` builds a `DateTime` from a year, a month, a day and a zone, and `fromIso` reads \
            a timestamp such as `2024-03-01T12:00:00Z`. `sleep` waits for a `Duration`.",
};

/// `Core\Time::now`'s reference card — `rule:core-api/reference-card`.
const TIME_NOW_DOC: MethodDoc = MethodDoc {
    short: "Reads the current time from the system clock. It replaces PHP's `time`, `microtime` \
            and `date_create`. Use `->in($zone)` to get the date and time of day in a zone.",
    params: &[],
    ret: "The current `Instant`, to the nanosecond.",
    errors: &[],
};

/// `Core\Time::monotonic`'s reference card — `rule:core-api/reference-card`.
const TIME_MONOTONIC_DOC: MethodDoc = MethodDoc {
    short: "Reads a clock that only moves forward, for measuring how long something takes. It \
            replaces PHP's `hrtime`. Subtract two readings to get the time between them.",
    params: &[],
    ret: "A `Duration` since a fixed starting point. A later reading is never smaller than an \
          earlier one.",
    errors: &[],
};

/// `Core\Time::sleep`'s reference card — `rule:core-api/reference-card`.
const TIME_SLEEP_DOC: MethodDoc = MethodDoc {
    short: "Waits for `$d`, replacing `sleep`, `usleep`, `time_nanosleep` and \
            `time_sleep_until` — parking the task rather than blocking the core, so a \
            neighbour on the same core runs meanwhile; a request cancelled mid-sleep stops \
            here.",
    params: &[ParamDoc {
        name: "d",
        desc: "How long to wait; zero or negative returns at once.",
        shape: &[],
    }],
    ret: "Nothing.",
    errors: &[],
};

/// `Core\Time::fromEpoch`'s reference card — `rule:core-api/reference-card`.
const TIME_FROM_EPOCH_DOC: MethodDoc = MethodDoc {
    short: "Builds the instant at a Unix timestamp, replacing `DateTime::setTimestamp`; the \
            `nanos` option is added after the second, so `-1` with `{nanos: 1}` is one \
            nanosecond after `-1`, not before it.",
    params: &[
        ParamDoc {
            name: "seconds",
            desc: "Whole seconds since `1970-01-01T00:00:00Z`; negative before it.",
            shape: &[],
        },
        ParamDoc {
            name: "nanos",
            desc: "Nanoseconds to add on top of the second, below `1000000000`; the default is \
                   `0`.",
            shape: &[],
        },
    ],
    ret: "The `Instant`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`nanos` is not below `1000000000`, or `$seconds` is outside \
               `-377705023201..=253402207200`. That is the time from `-9999-01-02T01:59:59Z` to \
               `9999-12-30T22:00:00Z`.",
    }],
};

/// `Core\Time::fromIso`'s reference card — `rule:core-api/reference-card`.
const TIME_FROM_ISO_DOC: MethodDoc = MethodDoc {
    short: "Reads an ISO-8601 / RFC 3339 timestamp that carries its own offset — \
            `2024-03-01T12:00:00Z`, `2024-03-01T13:00:00+01:00` — which is all of `strtotime` \
            and `DateTime::__construct` that survives: a civil time with no offset is \
            `Time::parse` with a zone, and a relative expression is a typed call.",
    params: &[ParamDoc {
        name: "text",
        desc: "The timestamp text, with a date, a time and a `Z` or `±HH:MM` offset.",
        shape: &[],
    }],
    ret: "The `Instant` the text names.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "`$text` is not such a timestamp, with the component it stopped at named. It is \
               also thrown for a time outside `-9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z`.",
    }],
};

/// `Core\Time::parse`'s reference card — `rule:core-api/reference-card`.
const TIME_PARSE_DOC: MethodDoc = MethodDoc {
    short: "Reads a civil date and time through a CLDR pattern and places it in `$zone`, \
            replacing `DateTime::createFromFormat` and `strptime`; a `$format` written as a \
            literal is read while checking, so a malformed one is `E0769` rather than a \
            throw. A field the pattern does not name is left at the start of its range.",
    params: &[
        ParamDoc {
            name: "text",
            desc: "The text to read; it stays data, so nothing of it carries a qualifier into \
                   the answer.",
            shape: &[],
        },
        ParamDoc {
            name: "format",
            desc: "A CLDR pattern of civil fields only — `yyyy-MM-dd HH:mm` — since the zone is \
                   `$zone`; a grammar, so a `tainted` one is refused.",
            shape: &[],
        },
        ParamDoc {
            name: "zone",
            desc: "The zone the civil fields are read in; a time the zone skips resolves \
                   forward past the gap.",
            shape: &[],
        },
    ],
    ret: "The `DateTime` in `$zone`.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$format` does not compile — a field letter outside the implemented subset, \
                   an unterminated quote — or names a zone or offset field, which `$zone` \
                   already answers. Only the first half is `E0769` for a written literal: a \
                   zonal field is a pattern the grammar reads perfectly well and a rule of \
                   this member's own, so it throws however `$format` arrived.",
        },
        ErrorDoc {
            error: "ParseError",
            desc: "`$text` does not match `$format`: a literal that differs, a field with no \
                   digits, trailing text, or fields that together are not a real civil time. It is \
                     also thrown for a time outside \
                     `-9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z`.",
        },
    ],
};

/// `Core\Time::at`'s reference card — `rule:core-api/reference-card`.
const TIME_AT_DOC: MethodDoc = MethodDoc {
    short: "Builds a civil date and time in `$zone` from its fields, replacing `mktime`, \
            `gmmktime` and `DateTime::setDate`; the clock fields default to midnight, and a \
            time the zone skips — 02:30 on a spring-forward morning — resolves forward rather \
            than throwing.",
    params: &[
        ParamDoc {
            name: "year",
            desc: "The year, within `-9999..=9999`. The time as a whole must also lie between \
                   `-9999-01-02T01:59:59Z` and `9999-12-30T22:00:00Z`.",
            shape: &[],
        },
        ParamDoc {
            name: "month",
            desc: "The month, `1` to `12`.",
            shape: &[],
        },
        ParamDoc {
            name: "day",
            desc: "The day of the month, which must exist in that month.",
            shape: &[],
        },
        ParamDoc {
            name: "zone",
            desc: "The zone the fields are read in.",
            shape: &[],
        },
        ParamDoc {
            name: "hour",
            desc: "The hour, `0` to `23`; the default is `0`.",
            shape: &[],
        },
        ParamDoc {
            name: "minute",
            desc: "The minute, `0` to `59`; the default is `0`.",
            shape: &[],
        },
        ParamDoc {
            name: "second",
            desc: "The second, `0` to `59`; the default is `0`.",
            shape: &[],
        },
        ParamDoc {
            name: "nanos",
            desc: "The subsecond nanoseconds, below `1000000000`; the default is `0`.",
            shape: &[],
        },
    ],
    ret: "The `DateTime` in `$zone`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A field is outside its range, the fields together are not a date that exists \
               — `30` February — which is what `checkdate` used to answer, or the time lies \
               past either end of the range `$year` names.",
    }],
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
/// on each side of every call to save two words — [AGENTS.md](/AGENTS.md)'s
/// priority ordering spends memory on simplicity, not the reverse.
pub const DATE: CoreClass = CoreClass {
    name: DATE_NAME,
    doc: Some(&DATE_CARD),
    methods: &[CoreMethod {
        name: "at",
        names: &["y", "m", "d"],
        params: &[CoreTy::Int, CoreTy::Uint, CoreTy::Uint],
        defaults: &[],
        return_ty: CoreTy::Instance(DATE_NAME),
        symbol: "nvs_core_time_date_at",
        doc: Some(&DATE_AT_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "format",
            names: &["pattern"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_date_format",
            doc: Some(&DATE_FORMAT_DOC),
        },
        CoreMethod {
            name: "plus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_plus",
            doc: Some(&DATE_PLUS_DOC),
        },
        CoreMethod {
            name: "minus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_minus",
            doc: Some(&DATE_MINUS_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(DATE_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(DATE_NAME),
            symbol: "nvs_core_time_date_with",
            doc: Some(&DATE_WITH_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(DATE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_date_compare_to",
            doc: Some(&DATE_COMPARE_TO_DOC),
        },
    ],
    slots: &["year", "month", "day"],
    constants: &[],
};

/// `Core\Time\Date`'s class card — `rule:core-api/reference-card`.
const DATE_CARD: ClassDoc = ClassDoc {
    short: "A date with no time of day and no zone, such as a birthday or a due date. \
            `Core\\Time\\Date::at` builds one from a year, a month and a day. `plus` and `minus` \
            move it by days, months or years, and `format` turns it into text.",
};

/// `Core\Time\Date::at`'s reference card — `rule:core-api/reference-card`.
const DATE_AT_DOC: MethodDoc = MethodDoc {
    short: "Builds a zone-free civil date from its three fields — the one place a date that \
            does not exist throws, which is why there is no `checkdate`.",
    params: &[
        ParamDoc {
            name: "y",
            desc: "The year, within `-9999..=9999`.",
            shape: &[],
        },
        ParamDoc {
            name: "m",
            desc: "The month, `1` to `12`.",
            shape: &[],
        },
        ParamDoc {
            name: "d",
            desc: "The day of the month, which must exist in that month.",
            shape: &[],
        },
    ],
    ret: "The `Date`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A field is outside its range, or the three together are not a date that exists.",
    }],
};

/// `Core\Time\Date::format`'s reference card — `rule:core-api/reference-card`.
const DATE_FORMAT_DOC: MethodDoc = MethodDoc {
    short: "Renders the date through a CLDR pattern of date fields — `yyyy-MM-dd`, `EEEE, d \
            MMMM yyyy` — the same grammar `DateTime::format` takes, narrowed to what a date \
            carries.",
    params: &[ParamDoc {
        name: "pattern",
        desc: "A CLDR pattern naming only calendar fields; a grammar, so a `tainted` one is \
               refused.",
        shape: &[],
    }],
    ret: "The rendered text.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$pattern` does not compile, or names a time-of-day or zone field, which a date \
               would have to invent.",
    }],
};

/// `Core\Time\Date::plus`'s reference card — `rule:core-api/reference-card`.
const DATE_PLUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the date forward by `$count` calendar steps of `$unit`, with the same \
            clamping `DateTime::plus` applies: the last day of January plus a month is the \
            last day of February.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves back.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "`Unit::Day`, `Unit::Week`, `Unit::Month`, `Unit::Quarter` or `Unit::Year`.",
            shape: &[],
        },
    ],
    ret: "A new `Date`; the receiver is unchanged.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$unit` is smaller than `Unit::Day`, which does not move a date.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is past what a calendar span can hold, or the result lies outside \
                   `-9999..=9999`.",
        },
    ],
};

/// `Core\Time\Date::minus`'s reference card — `rule:core-api/reference-card`.
const DATE_MINUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the date back by `$count` calendar steps of `$unit` — `plus` with the count \
            negated.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves forward.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "`Unit::Day`, `Unit::Week`, `Unit::Month`, `Unit::Quarter` or `Unit::Year`.",
            shape: &[],
        },
    ],
    ret: "A new `Date`; the receiver is unchanged.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$unit` is smaller than `Unit::Day`, which does not move a date.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is past what a calendar span can hold, or the result lies outside \
                   `-9999..=9999`.",
        },
    ],
};

/// `Core\Time\Date::with`'s reference card — `rule:core-api/reference-card`.
const DATE_WITH_DOC: MethodDoc = MethodDoc {
    short: "Replaces any of the three fields and leaves the rest; a combination that is not a \
            date throws rather than clamps, since `2024-02-29` with `{year: 2023}` has no \
            answer that is not a guess.",
    params: &[
        ParamDoc {
            name: "year",
            desc: "The new year, within `-9999..=9999`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "month",
            desc: "The new month, `1` to `12`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "day",
            desc: "The new day of the month; omitted leaves the field alone.",
            shape: &[],
        },
    ],
    ret: "A new `Date`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "An option is outside its field's range, or the fields together are not a date \
               that exists.",
    }],
};

/// `Core\Time\Date::compareTo`'s reference card — `rule:core-api/reference-card`.
const DATE_COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Orders two dates on the calendar, as `Comparable` requires.",
    params: &[ParamDoc {
        name: "other",
        desc: "The date to compare against.",
        shape: &[],
    }],
    ret: "`-1` when the receiver is earlier than `$other`, `0` when they are the same day, `1` \
          when it is later.",
    errors: &[],
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
    doc: None,
    methods: &[CoreMethod {
        name: "at",
        names: &["hour", "minute"],
        params: &[CoreTy::Uint, CoreTy::Uint, CoreTy::Options(AT_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
        symbol: "nvs_core_time_of_day_at",
        doc: Some(&TIME_OF_DAY_AT_DOC),
    }],
    instance: &[
        CoreMethod {
            name: "format",
            names: &["pattern"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_time_of_day_format",
            doc: Some(&TIME_OF_DAY_FORMAT_DOC),
        },
        CoreMethod {
            name: "plus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_plus",
            doc: Some(&TIME_OF_DAY_PLUS_DOC),
        },
        CoreMethod {
            name: "minus",
            names: &["count", "unit"],
            params: &[CoreTy::Int, CoreTy::Enum(UNIT.name)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_minus",
            doc: Some(&TIME_OF_DAY_MINUS_DOC),
        },
        CoreMethod {
            name: "with",
            names: &[],
            params: &[CoreTy::Options(TIME_OF_DAY_WITH_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Instance(TIME_OF_DAY_NAME),
            symbol: "nvs_core_time_of_day_with",
            doc: Some(&TIME_OF_DAY_WITH_DOC),
        },
        CoreMethod {
            name: "compareTo",
            names: &["other"],
            params: &[CoreTy::Instance(TIME_OF_DAY_NAME)],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_time_of_day_compare_to",
            doc: Some(&TIME_OF_DAY_COMPARE_TO_DOC),
        },
    ],
    slots: &["hour", "minute", "second", "nanos"],
    constants: &[],
};

/// `Core\Time\TimeOfDay::at`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_AT_DOC: MethodDoc = MethodDoc {
    short: "Builds a zone-free wall-clock reading from its fields, with the two a clock \
            usually leaves off defaulting to zero.",
    params: &[
        ParamDoc {
            name: "hour",
            desc: "The hour, `0` to `23`.",
            shape: &[],
        },
        ParamDoc {
            name: "minute",
            desc: "The minute, `0` to `59`.",
            shape: &[],
        },
        ParamDoc {
            name: "second",
            desc: "The second, `0` to `59`; the default is `0`.",
            shape: &[],
        },
        ParamDoc {
            name: "nanos",
            desc: "The subsecond nanoseconds, below `1000000000`; the default is `0`.",
            shape: &[],
        },
    ],
    ret: "The `TimeOfDay`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A field is outside its range, so the four are not a time of day.",
    }],
};

/// `Core\Time\TimeOfDay::format`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_FORMAT_DOC: MethodDoc = MethodDoc {
    short: "Renders the clock reading through a CLDR pattern of time fields — `HH:mm:ss`, \
            `h:mm a` — the same grammar `DateTime::format` takes, narrowed to what a clock \
            carries.",
    params: &[ParamDoc {
        name: "pattern",
        desc: "A CLDR pattern naming only time-of-day fields; a grammar, so a `tainted` one \
               is refused.",
        shape: &[],
    }],
    ret: "The rendered text.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$pattern` does not compile, or names a calendar or zone field, which a clock \
               reading would have to invent.",
    }],
};

/// `Core\Time\TimeOfDay::plus`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_PLUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the reading forward by `$count` steps of `$unit`, wrapping within the day: \
            `23:30` plus an hour is `00:30`, since a time of day has no date for a carry to \
            go to — a step that carries a day is `DateTime::plus`.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves back.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "`Unit::Hour` or smaller.",
            shape: &[],
        },
    ],
    ret: "A new `TimeOfDay`; the receiver is unchanged.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$unit` is `Unit::Day` or larger, which does not move a time of day.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is past what a span can hold.",
        },
    ],
};

/// `Core\Time\TimeOfDay::minus`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_MINUS_DOC: MethodDoc = MethodDoc {
    short: "Moves the reading back by `$count` steps of `$unit`, wrapping within the day — \
            `plus` with the count negated.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many steps; negative moves forward.",
            shape: &[],
        },
        ParamDoc {
            name: "unit",
            desc: "`Unit::Hour` or smaller.",
            shape: &[],
        },
    ],
    ret: "A new `TimeOfDay`; the receiver is unchanged.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$unit` is `Unit::Day` or larger, which does not move a time of day.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is past what a span can hold.",
        },
    ],
};

/// `Core\Time\TimeOfDay::with`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_WITH_DOC: MethodDoc = MethodDoc {
    short: "Replaces any of the four clock fields and leaves the rest.",
    params: &[
        ParamDoc {
            name: "hour",
            desc: "The new hour, `0` to `23`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "minute",
            desc: "The new minute, `0` to `59`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "second",
            desc: "The new second, `0` to `59`; omitted leaves the field alone.",
            shape: &[],
        },
        ParamDoc {
            name: "nanos",
            desc: "The new subsecond nanoseconds, below `1000000000`; omitted leaves the field \
                   alone.",
            shape: &[],
        },
    ],
    ret: "A new `TimeOfDay`; the receiver is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "An option is outside its field's range.",
    }],
};

/// `Core\Time\TimeOfDay::compareTo`'s reference card — `rule:core-api/reference-card`.
const TIME_OF_DAY_COMPARE_TO_DOC: MethodDoc = MethodDoc {
    short: "Orders two clock readings within the day, as `Comparable` requires.",
    params: &[ParamDoc {
        name: "other",
        desc: "The reading to compare against.",
        shape: &[],
    }],
    ret: "`-1` when the receiver is earlier in the day than `$other`, `0` when they are the \
          same reading, `1` when it is later.",
    errors: &[],
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

/// A `Core\Time\Instant` at `at`, for a member **outside this module** holding
/// a [`SystemTime`] — spec § 14's `Core\IO::modifiedAt` and the `modifiedAt`
/// slot of `Core\IO\Metadata` are the two so far.
///
/// The seam exists so that [`INSTANT`]'s two-slot representation — the epoch
/// second and the subsecond nanosecond, always of the same sign — stays inside
/// this module: a caller elsewhere holds a clock reading and wants the language
/// type for it, and has no business knowing which integers that is made of.
///
/// `None` for a reading outside [`Timestamp`]'s range, which is roughly years 1
/// to 9999. A filesystem is free to hand back a modification time no calendar
/// has a name for — a corrupt inode, a `0xFFFF_FFFF` sentinel — and the caller
/// decides what to say about it, because this seam has no member to name in a
/// message and no answer that would not be invented.
pub(crate) fn instant_at_system_time(at: SystemTime) -> Option<Value> {
    Timestamp::try_from(at).ok().map(instant_built)
}

/// A civil date and time as an outside reader rendered it, carrying no zone —
/// the shape [`datetime_at`] and [`instant_at`] take.
///
/// The widths are a *renderer's* rather than [`civil`]'s, so that a caller
/// hands over what it read and every range check happens here. `nvs_db`'s
/// `TIMESTAMP` and `TIMESTAMPTZ` columns are the first of them, and
/// PostgreSQL's `24:00:00` — a reading no `Core\Time\TimeOfDay` has — is
/// refused by this module rather than by a driver that would otherwise have to
/// know these bounds to refuse it.
pub(crate) struct Civil {
    /// The astronomical year, so `1 BC` is `0`.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59.
    pub second: u8,
    /// The nanosecond within the second.
    pub nanosecond: u32,
}

/// A `Core\Time\Date` on that year, month and day, for a member **outside this
/// module** holding a rendered date — `rule:core-classes/db-column-types`'s `DATE` column is the
/// first.
///
/// `None` for a combination no calendar has, which is
/// [`instant_at_system_time`]'s answer and for its reason: the caller holds
/// the name of the thing that produced it, and this seam does not.
pub(crate) fn date_at(year: i32, month: u8, day: u8) -> Option<Value> {
    civil_date(year, month, day).map(date_built)
}

/// A `Core\Time\TimeOfDay` at that reading — [`date_at`]'s twin, for § 9's
/// `TIME` column.
///
/// `None` for a reading no clock has, PostgreSQL's `24:00:00` included.
pub(crate) fn time_of_day_at(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Option<Value> {
    civil_time(hour, minute, second, nanosecond).map(clock_built)
}

/// A `Core\Time\DateTime` reading `at` in the fixed zone `offset` seconds east
/// of UTC — § 9's zone-less `TIMESTAMP`, in the zone its connection declared.
///
/// The zone is an offset and never a name, so the stored id is
/// [`render_offset`]'s `±HH:MM[:SS]` and the reading is unambiguous: a fixed
/// offset has no gap or fold for a civil time to land in, which is why this
/// seam has no ambiguity policy to state.
///
/// `None` for [`Civil`]'s refusals, and for an `offset` that is not one.
pub(crate) fn datetime_at(at: &Civil, offset: i32) -> Option<Value> {
    zoned_at(at, offset).map(|at| datetime_built(&at))
}

/// A `Core\Time\Instant` at `at` read `offset` seconds east of UTC — § 9's
/// `TIMESTAMPTZ`, whose rendering carries that offset itself.
///
/// `None` on [`datetime_at`]'s conditions, plus a point outside
/// [`Timestamp`]'s range.
pub(crate) fn instant_at(at: &Civil, offset: i32) -> Option<Value> {
    zoned_at(at, offset).map(|at| instant_built(at.timestamp()))
}

/// [`date_at`] off a rendered date rather than off three numbers — `rule:core-classes/db-column-types`'s SQLite half, where a `date` column holds text because SQLite has no
/// type that holds anything else.
///
/// `None` for text that is not a date, which is what § 9's "throws on a value
/// that does not parse" is asked here: the caller holds the column's name and
/// this seam does not, exactly as [`date_at`]'s own `None` works.
///
/// The grammar is [`jiff`]'s ISO 8601 reader and not a second one written here,
/// so a cell this accepts is a cell `Core\Time` would have rendered.
pub(crate) fn date_of_text(text: &str) -> Option<Value> {
    let date: civil::Date = text.trim().parse().ok()?;
    date_at(
        i32::from(date.year()),
        u8::try_from(date.month()).ok()?,
        u8::try_from(date.day()).ok()?,
    )
}

/// [`date_of_text`]'s twin for § 9's `TIME` column.
pub(crate) fn time_of_day_of_text(text: &str) -> Option<Value> {
    let time: civil::Time = text.trim().parse().ok()?;
    time_of_day_at(
        u8::try_from(time.hour()).ok()?,
        u8::try_from(time.minute()).ok()?,
        u8::try_from(time.second()).ok()?,
        u32::try_from(time.subsec_nanosecond()).ok()?,
    )
}

/// [`date_of_text`]'s twin for § 9's zone-less `DATETIME`, read in the zone the
/// connection declared — `offset` is [`datetime_at`]'s, for its reason.
///
/// The separator is either ISO 8601's `T` or the space SQLite's own `datetime()`
/// writes, which [`jiff`] reads as one grammar; a rendering carrying an offset
/// is refused here rather than silently dropping it, because a column declared
/// `datetime` is § 9's zone-*less* row and an `Instant` is what carries one.
pub(crate) fn datetime_of_text(text: &str, offset: i32) -> Option<Value> {
    let at: civil::DateTime = text.trim().parse().ok()?;
    datetime_at(
        &Civil {
            year: i32::from(at.year()),
            month: u8::try_from(at.month()).ok()?,
            day: u8::try_from(at.day()).ok()?,
            hour: u8::try_from(at.hour()).ok()?,
            minute: u8::try_from(at.minute()).ok()?,
            second: u8::try_from(at.second()).ok()?,
            nanosecond: u32::try_from(at.subsec_nanosecond()).ok()?,
        },
        offset,
    )
}

/// The [`Zoned`] `at` names at a fixed offset, which is the whole of what the
/// two seams above share.
fn zoned_at(at: &Civil, offset: i32) -> Option<Zoned> {
    let date = civil_date(at.year, at.month, at.day)?;
    let time = civil_time(at.hour, at.minute, at.second, at.nanosecond)?;
    let zone = TimeZone::fixed(Offset::from_seconds(offset).ok()?);
    civil::DateTime::from_parts(date, time).to_zoned(zone).ok()
}

/// The [`civil::Date`] those three fields name, or `None` for a date no
/// calendar has — a year past this crate's range included.
fn civil_date(year: i32, month: u8, day: u8) -> Option<civil::Date> {
    civil::Date::new(
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    )
    .ok()
}

/// The [`civil::Time`] those four fields name, or `None` for a reading no
/// clock has.
fn civil_time(hour: u8, minute: u8, second: u8, nanosecond: u32) -> Option<civil::Time> {
    civil::Time::new(
        i8::try_from(hour).ok()?,
        i8::try_from(minute).ok()?,
        i8::try_from(second).ok()?,
        i32::try_from(nanosecond).ok()?,
    )
    .ok()
}

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

/// The `Instant` `text` names, read as the RFC 3339 timestamp
/// [`nvs_core_time_from_iso`] reads — `None` where it is not one.
///
/// The parse without the throw, because [`crate::json`]'s wire form of this
/// type is this spelling and a document carrying anything else is a bad
/// document: it belongs in the issue list beside every other field that did not
/// fit, rather than ending the decode where it was met.
pub(crate) fn instant_from_iso(text: &str) -> Option<Value> {
    text.parse::<Timestamp>().ok().map(instant_built)
}

/// `value`'s RFC 3339 rendering, where it is an `Instant` — `$i->toIso()`'s
/// answer, for [`crate::json`] to write the same spelling it reads.
///
/// `None` where the two slots hold a pair no [`Timestamp`] is made of, which is
/// [`instant_of`]'s own unreachable-from-source guard seen through an
/// [`Option`]: the caller is an encoder and has no member to name in a fault.
pub(crate) fn instant_iso(value: Value) -> Option<String> {
    Some(instant_of(&[value], 0, "toIso").ok()?.to_string())
}

/// The `Instant` in argument slot `at` — slot 0 for a receiver, any other slot
/// for an `Instant` *parameter*.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives.
pub(crate) fn instant_of(args: &[Value], at: usize, member: &str) -> Result<Timestamp, Fault> {
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

/// `rule:core-api/shape-rules` R4 for every class here but `Duration`, which has
/// [`overflowed`]'s one sentence: a result past what the type can hold throws
/// rather than wrapping. `member` is the whole `Core\…::name` label, since
/// four classes reach this.
fn out_of_range(member: &str, why: &str) -> Fault {
    Fault::thrown(format!("{member}(): {why}"))
}

/// Why a member throws for a time past either end of the timestamp range. It
/// is the same sentence in every member that can leave the range, and it
/// replaces jiff's own, which names a parameter the program never wrote.
const PAST_THE_RANGE: &str = "the time is outside -9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z";

/// The `string` in argument slot `at`, for a member that takes text.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member, for the reason [`count`] gives, and
/// only that one: a `string` is guaranteed-valid UTF-8 (`rule:types/bytes`), and the tag
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
    // A `string` is guaranteed-valid UTF-8 (`rule:types/bytes`) and the tag is what says
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

/// The `Zone` in argument slot `at` as **seconds east of UTC, right now** —
/// what a database connection's declared zone is, per `rule:core-classes/db-column-types`.
///
/// A zone is not an offset and [`nvs_core_time_zone_offset_at`] takes an
/// instant for exactly that reason, so this picks one: the moment the
/// connection is opened. That is the honest reading of what the number is for
/// — it is sent to the server as a numeric offset for the length of the
/// session, because a named zone needs `mysql.time_zone` populated and usually
/// is not — and a session that outlives a DST change reads the offset it
/// opened with, which is the same thing the `time_zone` field of a
/// `[db.<name>]` block already means: that field accepts an offset spelling
/// and nothing else.
///
/// # Errors
///
/// [`zone_of`]'s, unchanged.
pub(crate) fn zone_offset_now(args: &[Value], at: usize, member: &str) -> Result<i32, Fault> {
    let zone = zone_of(args, at, member)?;
    Ok(zone.to_offset(Timestamp::now()).seconds())
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

/// Microseconds on [`MONOTONIC_ORIGIN`]'s timeline, for a native member that
/// measures rather than dates — [`crate::ratelimit`]'s per-core tier is the one
/// caller today.
///
/// The wall clock is deliberately not offered here: a limiter reading one would
/// refuse for as long as an NTP step moved it backwards, which is a rate limit
/// nobody configured. A monotonic reading cannot move backwards and cannot be
/// stepped, and an interval is all GCRA reads.
pub(crate) fn monotonic_micros() -> i128 {
    let origin = MONOTONIC_ORIGIN.get_or_init(std::time::Instant::now);
    i128::try_from(origin.elapsed().as_micros()).unwrap_or(i128::MAX)
}

/// `rule:testing/determinism-declared-on-the-test`'s `at:` reading — an RFC 3339 timestamp such as
/// `2026-01-01T00:00:00Z` — as nanoseconds since the Unix epoch, or `None` for
/// text that is not one.
///
/// Here rather than in the test runner because this module owns every
/// conversion between an instant and its text (`jiff` is its dependency and
/// nobody else's), so the grammar `#[Test(at: …)]` accepts is the grammar
/// `Instant::toIso` emits, with nothing keeping two readings in step.
#[must_use]
pub fn fixed_clock_nanos(text: &str) -> Option<i128> {
    text.parse::<Timestamp>().ok().map(Timestamp::as_nanosecond)
}

/// The instant `nanos` nanoseconds after the Unix epoch names, or `None` for a
/// count outside the representable range of about ±9999 years.
///
/// The one place a fixed clock's number becomes an instant again, so
/// `Core\Time::now` and `Core\Test::advance` cannot disagree about which counts
/// are readable.
///
/// **Built through `Timestamp::new`, never `Timestamp::from_nanosecond`.**
/// `jiff`'s `from_nanosecond` checks only that the seconds fit an `i64`, so a
/// count past year 9999 comes back `Ok` with an instant outside the range,
/// which a debug build then panics on and a release build carries on with.
/// `Timestamp::new` checks the range itself.
pub(crate) fn instant_at_nanos(nanos: i128) -> Option<Timestamp> {
    const NANOS_PER_SECOND: i128 = 1_000_000_000;
    let second = i64::try_from(nanos / NANOS_PER_SECOND).ok()?;
    let subsec = i32::try_from(nanos % NANOS_PER_SECOND).ok()?;
    Timestamp::new(second, subsec).ok()
}

/// The wall clock `ctx` reads: `rule:testing/determinism-declared-on-the-test`'s fixed one where a
/// `#[Test(at: …)]` armed it, and the host's otherwise.
///
/// `None` only for a fixed reading outside the representable range, which
/// `Core\Test::advance` refuses to store — so it is a state no program can
/// reach, kept in the signature rather than papered over because the two
/// callers want different things said about it.
///
/// **Every wall-clock reading in `Core` goes through here**, which today is
/// `Core\Time::now`, `Core\Uuid::v7`'s timestamp half and
/// `Core\Signature::verify`'s expiry judgement. A clock that some
/// readings honoured and others did not would make a test's reproducibility
/// depend on which members it happened to call, which is the same argument
/// `crate::random::draw` makes for the generator.
pub(crate) fn wall_clock(ctx: &Ctx) -> Option<Timestamp> {
    match ctx.fixed_clock() {
        None => Some(Timestamp::now()),
        Some(nanos) => instant_at_nanos(nanos),
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Time::now(): Instant` — the wall clock, which replaces `time`,
    /// `microtime` and `date_create` at once.
    ///
    /// The one member `rule:testing/determinism-declared-on-the-test`'s fixed clock reaches. A context with one
    /// armed answers that reading instead of the host's; every context outside
    /// a `#[Test(at: …)]` has none, so the cost on the ordinary path is one
    /// predictable not-taken branch and the fixed reading is unreachable from a
    /// program that is not a test.
    ///
    /// `Core\Time::monotonic` is deliberately **not** fixed with it: § 12
    /// declares a wall clock, and a monotonic reading is for measuring an
    /// interval that really elapsed. Freezing it would make a test's own
    /// timing measurements answer zero, which is a different decision and one
    /// nothing has asked for.
    fn nvs_core_time_now(ctx, args: [0]) {
        let _ = args;
        // unreachable from source: both writers of the fixed clock prove the
        // value representable before storing it — the test runner parses an
        // RFC 3339 `Timestamp` and reports a malformed `at:` against the test
        // that declared it, and `Core\Test::advance` refuses a move that would
        // leave the range rather than making one. This is what keeps those two
        // the only writers, in the sense a `debug_assert!` is, and there is no
        // program to write against it.
        let at = wall_clock(ctx).ok_or_else(|| {
            Fault::fatal(
                "Core\\Time::now found a fixed clock outside the representable range".to_owned(),
            )
        })?;
        Ok(instant_built(at))
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
    /// runs while this one waits (`rule:http-server/a-core-is-never-blocked-on-a-syscall`). It is also what makes a
    /// `Core\Task::all` under a `limit` observable at all: with a blocking
    /// sleep no two children ever overlap.
    ///
    /// [`Woken::Cancelled`] is the group cancelling this child mid-sleep. The
    /// member stops the request there and then, through [`Ctx::cancel`], which
    /// is `rule:concurrency/cancellation-runs-no-user-code`'s teardown reached by `rule:errors/propagation`'s return status — the
    /// only way it can be reached from a frame that is `extern "C"`.
    ///
    /// With no host on the thread the wait still has to happen, and blocking is
    /// the right answer: nothing else is running on this core.
    ///
    /// **Inside a connection isolate the sleep is the connection's**: it goes
    /// through [`nvs_runtime::peer::sleep`], which ends it at the connection's
    /// drain deadline at the latest and wakes it when the drain begins, so a
    /// program that only sends learns of a shutdown at its next `send` rather
    /// than when a long sleep ends (`rule:concurrency/a-drain-closes-a-connection-cleanly`).
    fn nvs_core_time_sleep(ctx, args: [1]) {
        let nanos = nanos_of(args, 0, "sleep")?;
        if nanos > 0 {
            let duration = std::time::Duration::from_nanos(nanos.unsigned_abs());
            match nvs_runtime::peer::sleep(ctx, duration) {
                Woken::Cancelled => return Err(ctx.cancel()),
                Woken::Elapsed => {}
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
        // The range is the timestamp's own, named in seconds because that is
        // what the caller passed. `nanos` cannot push an accepted second past
        // it: the last representable instant is a whole second plus
        // 999999999 nanoseconds, so the second failure is unreachable too.
        let outside = || {
            out_of_range(
                r"Core\Time::fromEpoch",
                "`seconds` is outside -377705023201..=253402207200",
            )
        };
        let at = Timestamp::from_second(seconds).map_err(|_| outside())?;
        #[expect(
            clippy::cast_possible_wrap,
            reason = "the bound checked one line above puts `nanos` below \
                      1e9, which is well inside `i64`"
        )]
        let offset = SignedDuration::from_nanos(nanos as i64);
        at.checked_add(offset)
            .map(instant_built)
            .map_err(|_| outside())
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
    /// site's. For a text that is not written as a timestamp, the sentence
    /// past the member's own name is `jiff`'s, since it says which component
    /// it stopped at and nothing here could say it better. A text that is
    /// written as one — a date, a time and an offset all read — and still
    /// fails names a time past either end of the timestamp range, and throws
    /// the range in the sentence [`nvs_core_time_at`] uses for the same bound.
    fn nvs_core_time_from_iso(_ctx, args: [1]) {
        let text = text_of(args, 0, "Core\\Time::fromIso")?;
        text.parse::<Timestamp>()
            .map(instant_built)
            .map_err(|err| {
                let written_as_one = Pieces::parse(text.as_bytes())
                    .is_ok_and(|read| read.time().is_some() && read.offset().is_some());
                let why = if written_as_one {
                    PAST_THE_RANGE.to_owned()
                } else {
                    err.to_string()
                };
                Fault::thrown_as(ThrownClass::Parse, format!("Core\\Time::fromIso(): {why}"))
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
    ///
    /// Adding a `SignedDuration` fails only past either end of the range.
    fn nvs_core_time_instant_plus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "plus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "plus")?);
        at.checked_add(by)
            .map(instant_built)
            .map_err(|_| out_of_range(r"Core\Time\Instant::plus", PAST_THE_RANGE))
    }
}

nvs_runtime::nvs_helper! {
    /// `$i->minus(Duration $d): Instant`.
    fn nvs_core_time_instant_minus(_ctx, args: [2]) {
        let at = instant_of(args, 0, "minus")?;
        let by = SignedDuration::from_nanos(nanos_of(args, 1, "minus")?);
        at.checked_sub(by)
            .map(instant_built)
            .map_err(|_| out_of_range(r"Core\Time\Instant::minus", PAST_THE_RANGE))
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
    /// (`rule:classes/comparable`).
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
        // refuses it: two ways to build one value is exactly what `rule:core-api/shape-rules`
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
    /// **Argument 0 is [`crate::registry::PREPARED_MEMBERS`]' word**, ahead of
    /// the receiver: [`crate::cldr::PREPARED_PATTERN`] for a pattern the call
    /// site wrote, which this core compiles once and keeps, and
    /// [`crate::cldr::PREPARED_NONE`] for one the program assembled, which is
    /// compiled here and kept nowhere. This module's own docs own that split.
    fn nvs_core_time_datetime_format(_ctx, args: [3]) {
        let ready = crate::cldr::prepared_arg(&args[0]);
        let at = zoned_of(args, 1, "format")?;
        let pattern = text_of(args, 2, "Core\\Time\\DateTime::format")?;
        let pieces = crate::cldr::compiled(pattern, ready)
            .map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Logic,
                    format!("Core\\Time\\DateTime::format(): {why}"),
                )
            })?;
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

/// The `uint`-or-`null` in argument slot `at` — an `rule:core-api/shape-rules` R2 option whose
/// omitted spelling is [`Const::Null`], for a member that has no in-range
/// sentinel to use instead.
///
/// A `uint` past `i64::MAX` reads as `i64::MAX`. Every caller narrows the
/// value to its field's width and throws its own out-of-range sentence, so
/// that `uint` gets the same catchable `RuntimeError` as any other number too
/// wide for the field.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member and the option, for the reason
/// [`count`] gives. `member` is the whole `Core\…::name` label, since every
/// `with` member reaches here.
fn optional(args: &[Value], at: usize, member: &str, option: &str) -> Result<Option<i64>, Fault> {
    if args[at].tag() == Some(nvs_runtime::Tag::Null) {
        return Ok(None);
    }
    args[at]
        .as_int()
        .or_else(|| {
            args[at]
                .as_uint()
                .map(|held| i64::try_from(held).unwrap_or(i64::MAX))
        })
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
    /// **Argument 0 is [`crate::registry::PREPARED_MEMBERS`]' word**, read
    /// exactly as [`nvs_core_time_datetime_format`] reads it. It says whether
    /// the *pattern* was written at the call site, never anything about the
    /// text being read, which is a request's to send.
    ///
    /// The zone is the third argument rather than something the pattern can
    /// name, which is why a zonal field in the pattern is refused: two answers
    /// for one question is what `rule:core-api/shape-rules` R20 leaves no room for. That refusal
    /// is [`crate::cldr::civil_fields_only`]'s and is made against the compiled
    /// *pattern*, before a byte of the text is read, which is what puts it on
    /// the `LogicError` side of the split below rather than on the
    /// `ParseError` one — a well-formed offset in the text is not the input
    /// failing to match.
    fn nvs_core_time_parse(_ctx, args: [4]) {
        let ready = crate::cldr::prepared_arg(&args[0]);
        let text = text_of(args, 1, "Core\\Time::parse")?;
        let pattern = text_of(args, 2, "Core\\Time::parse")?;
        let zone = zone_of(args, 3, "parse")?;
        // The two failures are different spec § 10 classes on purpose: a
        // pattern this call site wrote wrongly is a bug in the program, while
        // text that does not match a well-formed pattern is exactly "input did
        // not match a format this code declared".
        //
        // The zonal refusal is made here whatever the word said: it is a
        // refusal rather than a routing decision, and a prepared pattern
        // changes which of them costs work, never which of them is made.
        let pieces = crate::cldr::compiled(pattern, ready)
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
        // Compatible disambiguation resolves every gap and fold, so the one
        // failure left is an instant past either end of the timestamp range.
        zone.to_zoned(civil)
            .map(|at| datetime_built(&at))
            .map_err(|_| out_of_range(r"Core\Time::at", PAST_THE_RANGE))
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

/// The date `year-month-day`, or the throw `rule:core-api/shape-rules` R4 owes for a triple that
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
            .map_err(|why| {
                Fault::thrown_as(ThrownClass::Logic, format!("Core\\Time\\Date::format(): {why}"))
            })?;
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
            .map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Logic,
                    format!("Core\\Time\\TimeOfDay::format(): {why}"),
                )
            })?;
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

    /// The seams a rendered date or time arrives through refuse exactly the
    /// renderings no `Core\Time` type has, and accept the last value on the
    /// other side of each bound.
    ///
    /// Asserted on the components rather than on a built instance, so the case
    /// allocates nothing: what the four seams add over the landed builders is
    /// the range check, and this is all of it.
    #[test]
    fn a_rendering_no_core_time_type_has_is_refused_where_it_is_read() {
        assert!(civil_date(2024, 3, 5).is_some());
        assert!(civil_date(9999, 12, 31).is_some());
        assert!(civil_date(10_000, 1, 1).is_none());
        assert!(civil_date(2023, 2, 29).is_none());
        assert!(civil_date(2024, 13, 1).is_none());

        assert!(civil_time(23, 59, 59, 999_999_999).is_some());
        // PostgreSQL renders a `TIME` of `24:00:00`, and `Core\Time\TimeOfDay`
        // is the one type with no value for it — `rule:core-classes/db-column-types`'s row stops here
        // rather than folding to the midnight that follows it.
        assert!(civil_time(24, 0, 0, 0).is_none());
        assert!(civil_time(0, 60, 0, 0).is_none());
    }

    /// A civil rendering plus a declared offset is one point in time, and the
    /// offset is the whole of the difference — `rule:core-classes/db-column-types`'s zone-less
    /// `TIMESTAMP` read in the zone its connection declared.
    #[test]
    fn a_civil_rendering_is_read_at_the_offset_it_is_given() {
        let at = Civil {
            year: 2024,
            month: 3,
            day: 5,
            hour: 12,
            minute: 30,
            second: 15,
            nanosecond: 123_456_000,
        };
        let utc = zoned_at(&at, 0).expect("a civil rendering at UTC is a point in time");
        let east = zoned_at(&at, 2 * 3600).expect("and so is the same one two hours east");
        assert_eq!(
            utc.timestamp().as_second() - east.timestamp().as_second(),
            2 * 3600
        );
        assert_eq!(utc.timestamp().subsec_nanosecond(), 123_456_000);
        // A fixed offset has no IANA name, so the `DateTime` this builds
        // stores `render_offset`'s spelling — `ZONE`'s rule for a zone that is
        // an offset, reached here without allocating the instance.
        assert_eq!(east.time_zone().iana_name(), None);
        assert_eq!(render_offset(east.offset().seconds()), "+02:00");
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

    /// `Core\Time::at` called the way a compiled call site calls it, with the
    /// fields in `month, day, hour, minute, second, nanos` order. It answers
    /// the built `DateTime`'s seconds, nanoseconds and zone id, or the sentence
    /// it threw.
    fn built_at(year: i64, fields: [u64; 6], zone: &str) -> Result<(i64, i64, String), String> {
        let [month, day, hour, minute, second, nanos] = fields;
        let mut ctx = Ctx::buffered();
        let zone = zone_built(zone);
        let args = [
            Value::int(year),
            Value::uint(month),
            Value::uint(day),
            zone,
            Value::uint(hour),
            Value::uint(minute),
            Value::uint(second),
            Value::uint(nanos),
        ];
        let answer = match nvs_runtime::call(nvs_core_time_at, &mut ctx, &args) {
            Ok(built) => {
                let held = crate::instance::receiver(built, &DATETIME, "test")
                    .expect("`at` answers a `DateTime`");
                let read = (
                    crate::instance::slot(held, DATETIME_SECONDS_SLOT)
                        .as_int()
                        .expect("the seconds slot is an int"),
                    crate::instance::slot(held, DATETIME_NANOS_SLOT)
                        .as_int()
                        .expect("the nanos slot is an int"),
                    crate::instance::slot(held, DATETIME_ZONE_SLOT)
                        .as_text()
                        .expect("the zone slot is a text")
                        .to_owned(),
                );
                #[expect(
                    unsafe_code,
                    reason = "this frame owns the `DateTime` the member answered"
                )]
                unsafe {
                    built.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        };
        #[expect(
            unsafe_code,
            reason = "this frame built the `Zone`, and the member borrowed its arguments"
        )]
        unsafe {
            zone.release();
        }
        answer
    }

    /// The instant an RFC 3339 text names, as whole seconds.
    fn second_of(text: &str) -> i64 {
        text.parse::<Timestamp>()
            .expect("the test's own timestamp parses")
            .as_second()
    }

    /// `at` reads its fields in the zone it is given. A clock time the zone
    /// skips moves forward past the gap, and a clock time the zone repeats
    /// takes the earlier of the two. Every field's last accepted value and
    /// first refused one are asserted together.
    // covers: Core\Time::at
    #[test]
    fn at_reads_fields_in_its_zone_and_refuses_a_field_past_its_range() {
        assert_eq!(
            built_at(2024, [3, 5, 12, 30, 15, 500_000_000], "UTC"),
            Ok((
                second_of("2024-03-05T12:30:15Z"),
                500_000_000,
                "UTC".to_owned()
            ))
        );
        // 02:30 does not exist in Berlin on 2024-03-31, so the result is 03:30
        // summer time, which is 01:30 UTC.
        let skipped =
            built_at(2024, [3, 31, 2, 30, 0, 0], "Europe/Berlin").expect("a gap resolves");
        assert_eq!(skipped.0, second_of("2024-03-31T01:30:00Z"));
        assert_eq!(skipped.2, "Europe/Berlin");
        // 02:30 happens twice in Berlin on 2024-10-27; the first is 00:30 UTC.
        let repeated =
            built_at(2024, [10, 27, 2, 30, 0, 0], "Europe/Berlin").expect("a fold resolves");
        assert_eq!(repeated.0, second_of("2024-10-27T00:30:00Z"));

        for (year, fields, accepted) in [
            (2024, [2, 29, 0, 0, 0, 0], true),
            (2023, [2, 29, 0, 0, 0, 0], false),
            (2024, [12, 31, 0, 0, 0, 0], true),
            (2024, [13, 1, 0, 0, 0, 0], false),
            (2024, [1, 1, 23, 59, 59, 999_999_999], true),
            (2024, [1, 1, 24, 0, 0, 0], false),
            (2024, [1, 1, 0, 60, 0, 0], false),
            (2024, [1, 1, 0, 0, 60, 0], false),
            (2024, [1, 1, 0, 0, 0, 1_000_000_000], false),
            (2024, [1, 1, 0, 0, 0, u64::MAX], false),
            (2024, [u64::MAX, 1, 0, 0, 0, 0], false),
            // The last and first instants a timestamp holds, one second apart
            // from the first refused ones.
            (9999, [12, 30, 22, 0, 0, 0], true),
            (9999, [12, 30, 22, 0, 1, 0], false),
            (-9999, [1, 2, 1, 59, 59, 0], true),
            (-9999, [1, 2, 1, 59, 58, 0], false),
            (10_000, [1, 1, 0, 0, 0, 0], false),
            (-10_000, [1, 1, 0, 0, 0, 0], false),
        ] {
            match built_at(year, fields, "UTC") {
                Ok(_) => assert!(accepted, "{year} {fields:?} was accepted"),
                Err(sentence) => {
                    assert!(!accepted, "{year} {fields:?} was refused: {sentence}");
                    assert!(
                        sentence.starts_with(r"Core\Time::at(): "),
                        "the refusal names the member: {sentence}"
                    );
                }
            }
        }
    }

    /// `Core\Time::fromEpoch` called the way a compiled call site calls it. It
    /// answers the built `Instant`'s seconds and nanoseconds, or the sentence it
    /// threw.
    fn built_from_epoch(seconds: i64, nanos: u64) -> Result<(i64, i64), String> {
        let mut ctx = Ctx::buffered();
        let args = [Value::int(seconds), Value::uint(nanos)];
        match nvs_runtime::call(nvs_core_time_from_epoch, &mut ctx, &args) {
            Ok(built) => {
                let held = crate::instance::receiver(built, &INSTANT, "test")
                    .expect("`fromEpoch` answers an `Instant`");
                let read = (
                    crate::instance::slot(held, INSTANT_SECONDS_SLOT)
                        .as_int()
                        .expect("the seconds slot is an int"),
                    crate::instance::slot(held, INSTANT_NANOS_SLOT)
                        .as_int()
                        .expect("the nanos slot is an int"),
                );
                #[expect(
                    unsafe_code,
                    reason = "this frame owns the `Instant` the member answered"
                )]
                unsafe {
                    built.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        }
    }

    /// `fromEpoch` adds `nanos` after the second, accepts exactly the
    /// timestamp range at both ends, and refuses past it in its own sentence.
    // covers: Core\Time::fromEpoch
    #[test]
    fn from_epoch_adds_nanos_after_the_second_and_refuses_past_the_range_plainly() {
        assert_eq!(
            built_from_epoch(1_700_000_000, 0),
            Ok((second_of("2023-11-14T22:13:20Z"), 0))
        );
        // One nanosecond after `-1` is `-0.999999999`. The instant keeps its
        // two slots at one sign, so that is second `0` and nanos `-999999999`.
        assert_eq!(built_from_epoch(-1, 1), Ok((0, -999_999_999)));
        assert_eq!(
            built_from_epoch(253_402_207_200, 999_999_999),
            Ok((253_402_207_200, 999_999_999))
        );
        assert_eq!(
            built_from_epoch(-377_705_023_201, 0),
            Ok((-377_705_023_201, 0))
        );

        let past = r"Core\Time::fromEpoch(): `seconds` is outside -377705023201..=253402207200";
        for seconds in [253_402_207_201, -377_705_023_202, i64::MAX, i64::MIN] {
            assert_eq!(built_from_epoch(seconds, 0), Err(past.to_owned()));
        }
        for nanos in [1_000_000_000, u64::MAX] {
            assert_eq!(
                built_from_epoch(0, nanos),
                Err(r"Core\Time::fromEpoch(): `nanos` is a subsecond count, so it is below 1000000000".to_owned())
            );
        }
    }

    /// `Core\Time::fromIso` called the way a compiled call site calls it. It
    /// answers the built `Instant`'s seconds and nanoseconds, or the sentence it
    /// threw.
    fn built_from_iso(text: &str) -> Result<(i64, i64), String> {
        let mut ctx = Ctx::buffered();
        let args = [Value::str(NvsStr::new(text.as_bytes()))];
        let answer = match nvs_runtime::call(nvs_core_time_from_iso, &mut ctx, &args) {
            Ok(built) => {
                let held = crate::instance::receiver(built, &INSTANT, "test")
                    .expect("`fromIso` answers an `Instant`");
                let read = (
                    crate::instance::slot(held, INSTANT_SECONDS_SLOT)
                        .as_int()
                        .expect("the seconds slot is an int"),
                    crate::instance::slot(held, INSTANT_NANOS_SLOT)
                        .as_int()
                        .expect("the nanos slot is an int"),
                );
                #[expect(
                    unsafe_code,
                    reason = "this frame owns the `Instant` the member answered"
                )]
                unsafe {
                    built.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        };
        for argument in args {
            #[expect(unsafe_code, reason = "the argument list owns the one reference")]
            unsafe {
                argument.release();
            }
        }
        answer
    }

    /// `fromIso` applies the offset, keeps the fraction, accepts the last
    /// instant of the range, and throws a time past either end in its own
    /// sentence while a text that is not a timestamp keeps `jiff`'s.
    // covers: Core\Time::fromIso
    #[test]
    fn from_iso_applies_the_offset_and_refuses_past_the_range_plainly() {
        assert_eq!(
            built_from_iso("2024-03-01T13:00:00+01:00"),
            Ok((second_of("2024-03-01T12:00:00Z"), 0))
        );
        assert_eq!(
            built_from_iso("2024-03-01T12:00:00.5Z"),
            Ok((second_of("2024-03-01T12:00:00Z"), 500_000_000))
        );
        assert_eq!(
            built_from_iso("9999-12-30T22:00:00Z"),
            Ok((253_402_207_200, 0))
        );

        let past = r"Core\Time::fromIso(): the time is outside -9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z";
        for text in ["9999-12-31T23:59:59Z", "-009999-01-01T00:00:00Z"] {
            assert_eq!(built_from_iso(text), Err(past.to_owned()));
        }
        for text in [
            "2024-03-01T12:00:00",
            "2024-02-30T12:00:00Z",
            "tomorrow",
            "",
        ] {
            let said = built_from_iso(text).expect_err("not a timestamp");
            assert!(said.starts_with(r"Core\Time::fromIso(): "), "{said}");
            assert_ne!(said, past, "{text:?} is not a range failure");
        }
    }

    /// `Core\Time::parse` called the way a compiled call site calls it with a
    /// computed pattern. It answers the built `DateTime`'s seconds and zone id,
    /// or the sentence it threw.
    fn built_parse(text: &str, format: &str, zone: &str) -> Result<(i64, String), String> {
        let mut ctx = Ctx::buffered();
        let args = [
            Value::null(),
            Value::str(NvsStr::new(text.as_bytes())),
            Value::str(NvsStr::new(format.as_bytes())),
            zone_built(zone),
        ];
        let answer = match nvs_runtime::call(nvs_core_time_parse, &mut ctx, &args) {
            Ok(built) => {
                let held = crate::instance::receiver(built, &DATETIME, "test")
                    .expect("`parse` answers a `DateTime`");
                let read = (
                    crate::instance::slot(held, DATETIME_SECONDS_SLOT)
                        .as_int()
                        .expect("the seconds slot is an int"),
                    crate::instance::slot(held, DATETIME_ZONE_SLOT)
                        .as_text()
                        .expect("the zone slot is a text")
                        .to_owned(),
                );
                #[expect(
                    unsafe_code,
                    reason = "this frame owns the `DateTime` the member answered"
                )]
                unsafe {
                    built.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        };
        for argument in args {
            #[expect(unsafe_code, reason = "the argument list owns the one reference")]
            unsafe {
                argument.release();
            }
        }
        answer
    }

    /// `parse` reads the fields in its zone and starts an unnamed field at the
    /// bottom of its range. A time past either end of the timestamp range
    /// throws the sentence `at` uses, and a field wider than an `i64` is out
    /// of range rather than an overflow.
    // covers: Core\Time::parse
    #[test]
    fn parse_reads_in_its_zone_and_refuses_past_the_range_plainly() {
        assert_eq!(
            built_parse("05.06.2024 14:30", "dd.MM.yyyy HH:mm", "Europe/Berlin"),
            Ok((
                second_of("2024-06-05T12:30:00Z"),
                "Europe/Berlin".to_owned()
            ))
        );
        assert_eq!(
            built_parse("05.06.2024", "dd.MM.yyyy", "UTC"),
            Ok((second_of("2024-06-05T00:00:00Z"), "UTC".to_owned()))
        );
        // 02:30 does not exist in Berlin on 2024-03-31, so the result is 03:30
        // summer time, which is 01:30 UTC.
        assert_eq!(
            built_parse("2024-03-31 02:30", "yyyy-MM-dd HH:mm", "Europe/Berlin").map(|read| read.0),
            Ok(second_of("2024-03-31T01:30:00Z"))
        );
        assert_eq!(
            built_parse("9999-12-30 22:00", "uuuu-MM-dd HH:mm", "UTC").map(|read| read.0),
            Ok(253_402_207_200)
        );

        let past =
            r"Core\Time::parse(): the time is outside -9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z";
        for (text, zone) in [
            ("9999-12-30 22:01", "UTC"),
            ("9999-12-30 23:30", "Europe/Berlin"),
            ("-9999-01-02 01:58", "UTC"),
        ] {
            assert_eq!(
                built_parse(text, "uuuu-MM-dd HH:mm", zone),
                Err(past.to_owned())
            );
        }
        assert_eq!(
            built_parse(&"9".repeat(40), &"y".repeat(40), "UTC"),
            Err(r"Core\Time::parse(): year out of range".to_owned())
        );
        for (text, format) in [
            ("2024-06-05T14:30", "yyyy-MM-dd HH:mm"),
            ("2024", "yyyy 'x"),
        ] {
            let said = built_parse(text, format, "UTC").expect_err("does not match");
            assert!(said.starts_with(r"Core\Time::parse(): "), "{said}");
            assert_ne!(said, past, "{text:?} is not a range failure");
        }
    }

    /// `Core\Time::sleep` for `nanos`, called the way a compiled call site
    /// calls it with no host on the thread. It answers how long the call took.
    fn slept(nanos: i64) -> std::time::Duration {
        let mut ctx = Ctx::buffered();
        let args = [built(nanos)];
        let began = std::time::Instant::now();
        let answer = nvs_runtime::call(nvs_core_time_sleep, &mut ctx, &args)
            .expect("`sleep` throws nothing");
        let took = began.elapsed();
        assert_eq!(
            answer.tag_byte(),
            Value::null().tag_byte(),
            "`sleep` answers nothing"
        );
        for argument in args {
            #[expect(unsafe_code, reason = "the argument list owns the one reference")]
            unsafe {
                argument.release();
            }
        }
        took
    }

    /// `sleep` returns at once for zero and every negative duration, down to
    /// the most negative one, and a positive one is never short.
    // covers: Core\Time::sleep
    #[test]
    fn sleep_returns_at_once_at_or_below_zero_and_waits_above_it() {
        let at_once: std::time::Duration = [0, -1, -1_000_000_000, i64::MIN]
            .into_iter()
            .map(slept)
            .sum();
        assert!(
            at_once < std::time::Duration::from_millis(100),
            "{at_once:?}"
        );
        for nanos in [1, 2_000_000] {
            let took = slept(nanos);
            assert!(
                took >= std::time::Duration::from_nanos(nanos.unsigned_abs()),
                "{nanos} ns took {took:?}"
            );
        }
    }

    /// `Core\Time::now` called on `ctx` the way a compiled call site calls
    /// it. It answers the `Instant` as nanoseconds since the epoch.
    fn read_now(ctx: &mut Ctx) -> i128 {
        let built =
            nvs_runtime::call(nvs_core_time_now, ctx, &[]).expect("`now` throws nothing here");
        let at = instant_of(&[built], 0, "test").expect("`now` answers an `Instant`");
        #[expect(
            unsafe_code,
            reason = "this frame owns the `Instant` the member answered"
        )]
        unsafe {
            built.release();
        }
        at.as_nanosecond()
    }

    /// `Core\Time::monotonic` called the way a compiled call site calls it,
    /// on `ctx`. It answers the `Duration` in nanoseconds.
    fn read_monotonic(ctx: &mut Ctx) -> i64 {
        let built = nvs_runtime::call(nvs_core_time_monotonic, ctx, &[])
            .expect("`monotonic` throws nothing");
        let nanos = nanos_of(&[built], 0, "test").expect("`monotonic` answers a `Duration`");
        #[expect(
            unsafe_code,
            reason = "this frame owns the `Duration` the member answered"
        )]
        unsafe {
            built.release();
        }
        nanos
    }

    /// `now` reads the host's wall clock, between two readings taken around
    /// it, and answers the fixed reading instead once a test has armed one.
    // covers: Core\Time::now
    #[test]
    fn now_reads_the_host_clock_unless_a_fixed_one_is_armed() {
        let mut ctx = Ctx::buffered();
        let before = Timestamp::now().as_nanosecond();
        let read = read_now(&mut ctx);
        let after = Timestamp::now().as_nanosecond();
        assert!(
            before <= read && read <= after,
            "{before} <= {read} <= {after}"
        );

        let fixed = i128::from(second_of("2024-03-01T12:00:00Z")) * 1_000_000_000 + 5;
        ctx.set_fixed_clock(fixed);
        assert_eq!(read_now(&mut ctx), fixed);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert_eq!(read_now(&mut ctx), fixed, "a fixed clock does not move");
    }

    /// `monotonic` never goes backwards, counts a real wait in full, and is
    /// not frozen by the fixed clock that freezes `now`.
    // covers: Core\Time::monotonic
    #[test]
    fn monotonic_never_goes_back_and_counts_a_real_wait() {
        let mut ctx = Ctx::buffered();
        let mut last = read_monotonic(&mut ctx);
        assert!(last >= 0, "{last}");
        for _ in 0..1000 {
            let next = read_monotonic(&mut ctx);
            assert!(next >= last, "{next} after {last}");
            last = next;
        }
        ctx.set_fixed_clock(0);
        let began = read_monotonic(&mut ctx);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let waited = read_monotonic(&mut ctx) - began;
        assert!(waited >= 2_000_000, "{waited} ns");
    }

    /// A `Date` step called the way a compiled call site calls it: `step` is
    /// `plus` or `minus`, and `unit` is a `Core\Unit` case index. It answers
    /// the date the step reached, or the sentence it threw.
    fn date_step(
        step: nvs_runtime::NvsFn,
        from: (i16, i8, i8),
        count: i64,
        unit: i64,
    ) -> Result<civil::Date, String> {
        date_called(step, from, &[Value::int(count), Value::int(unit)])
    }

    /// `member` called on the `Date` `from` with the scalar arguments `rest`.
    /// It answers the `Date` the member returned, or the sentence it threw.
    fn date_called(
        member: nvs_runtime::NvsFn,
        from: (i16, i8, i8),
        rest: &[Value],
    ) -> Result<civil::Date, String> {
        let mut ctx = Ctx::buffered();
        let receiver = date_built(civil::date(from.0, from.1, from.2));
        let args: Vec<Value> = std::iter::once(receiver)
            .chain(rest.iter().copied())
            .collect();
        let answer = match nvs_runtime::call(member, &mut ctx, &args) {
            Ok(moved) => {
                let read = date_of(&[moved], 0, "test").expect("a step answers a `Date`");
                #[expect(unsafe_code, reason = "this frame owns the `Date` the member answered")]
                unsafe {
                    moved.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        };
        #[expect(
            unsafe_code,
            reason = "this frame built the `Date`, and the member borrowed its arguments"
        )]
        unsafe {
            receiver.release();
        }
        answer
    }

    /// `minus` is `plus` with the count negated, in every unit a date takes,
    /// including the clamp to a shorter month's last day. A unit smaller than
    /// a day throws, and so does the one count whose negation no `int` holds.
    /// The last day the calendar reaches going back and the first step it
    /// refuses are asserted together.
    // covers: Core\Time\Date::minus, Core\Time\Date::plus
    #[test]
    fn date_minus_is_plus_negated_and_refuses_a_unit_or_count_it_cannot_take() {
        const DAY: i64 = 6;
        const MONTH: i64 = 8;
        const QUARTER: i64 = 9;
        const YEAR: i64 = 10;
        let minus = nvs_core_time_date_minus as nvs_runtime::NvsFn;
        let plus = nvs_core_time_date_plus as nvs_runtime::NvsFn;
        let march_end = (2024, 3, 31);
        assert_eq!(
            date_step(minus, march_end, 1, MONTH),
            Ok(civil::date(2024, 2, 29))
        );
        assert_eq!(
            date_step(minus, march_end, 1, QUARTER),
            Ok(civil::date(2023, 12, 31))
        );
        assert_eq!(
            date_step(minus, (2024, 2, 29), 1, YEAR),
            Ok(civil::date(2023, 2, 28))
        );
        for unit in DAY..=YEAR {
            for count in [-400, -13, -1, 0, 1, 7, 13, 400] {
                assert_eq!(
                    date_step(minus, march_end, count, unit),
                    date_step(plus, march_end, -count, unit),
                    "{count} of unit {unit}"
                );
            }
        }
        for unit in 0..DAY {
            let refused = date_step(minus, march_end, 1, unit).expect_err("a clock unit");
            assert!(refused.contains("smaller than `Unit::Day`"), "{refused}");
        }
        let refused = date_step(minus, march_end, i64::MIN, DAY).expect_err("no negation");
        assert!(
            refused.contains("past what a calendar span can hold"),
            "{refused}"
        );
        assert_eq!(
            date_step(minus, (-9999, 1, 2), 1, DAY),
            Ok(civil::date(-9999, 1, 1))
        );
        let refused = date_step(minus, (-9999, 1, 1), 1, DAY).expect_err("the first day");
        assert!(
            refused.starts_with(r"Core\Time\Date::minus(): "),
            "{refused}"
        );
    }

    /// `with` replaces only the fields it is given, and throws for a
    /// combination that is not a date instead of moving to a nearby one. Each
    /// field's last accepted value and first refused one are asserted
    /// together, and a `uint` past the `int` range throws the same sentence
    /// as any other number too wide for the field.
    // covers: Core\Time\Date::with
    #[test]
    fn date_with_replaces_named_fields_and_refuses_a_combination_that_is_not_a_date() {
        let with =
            |from: (i16, i8, i8), year: Option<i64>, month: Option<u64>, day: Option<u64>| {
                date_called(
                    nvs_core_time_date_with,
                    from,
                    &[
                        year.map_or_else(Value::null, Value::int),
                        month.map_or_else(Value::null, Value::uint),
                        day.map_or_else(Value::null, Value::uint),
                    ],
                )
            };
        let leap = (2024, 2, 29);
        assert_eq!(with(leap, None, None, None), Ok(civil::date(2024, 2, 29)));
        assert_eq!(
            with(leap, Some(2028), None, None),
            Ok(civil::date(2028, 2, 29))
        );
        assert_eq!(
            with(leap, None, Some(12), Some(1)),
            Ok(civil::date(2024, 12, 1))
        );
        assert!(
            with(leap, Some(2023), None, None).is_err(),
            "2023-02-29 is not a date"
        );
        // 15 January exists in every year, so only the named field decides.
        let january = (2024, 1, 15);
        for (year, month, day, accepted) in [
            (Some(9999), None, None, true),
            (Some(10000), None, None, false),
            (Some(-9999), None, None, true),
            (Some(-10000), None, None, false),
            (None, Some(12), None, true),
            (None, Some(13), None, false),
            (None, Some(1), None, true),
            (None, Some(0), None, false),
            (None, None, Some(1), true),
            (None, None, Some(0), false),
            (None, None, Some(31), true),
            (None, None, Some(32), false),
        ] {
            assert_eq!(
                with(january, year, month, day).is_ok(),
                accepted,
                "{year:?}-{month:?}-{day:?}"
            );
        }
        for month in [u64::try_from(i64::MAX).expect("fits"), u64::MAX] {
            assert_eq!(
                with(leap, None, Some(month), None),
                Err(r"Core\Time\Date::with(): `month` is outside what that field can hold".into())
            );
        }
    }

    /// `compareTo` orders dates by year, then month, then day, and answers
    /// only `-1`, `0` or `1`. The table is in calendar order, so every pair's
    /// answer is the order of the two positions.
    // covers: Core\Time\Date::compareTo
    #[test]
    fn date_compare_to_is_the_calendar_order() {
        let table: [(i16, i8, i8); 9] = [
            (-9999, 1, 1),
            (-1, 12, 31),
            (0, 1, 1),
            (2023, 12, 31),
            (2024, 1, 1),
            (2024, 1, 31),
            (2024, 2, 1),
            (2024, 2, 29),
            (9999, 12, 31),
        ];
        for (i, left) in table.iter().enumerate() {
            for (j, right) in table.iter().enumerate() {
                let mut ctx = Ctx::buffered();
                let args = [
                    date_built(civil::date(left.0, left.1, left.2)),
                    date_built(civil::date(right.0, right.1, right.2)),
                ];
                let answer = nvs_runtime::call(nvs_core_time_date_compare_to, &mut ctx, &args)
                    .expect("any two dates compare")
                    .as_int();
                let expected = match i.cmp(&j) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                assert_eq!(answer, Some(expected), "{left:?} against {right:?}");
                for held in args {
                    #[expect(
                        unsafe_code,
                        reason = "this frame built both `Date`s, and the member borrowed them"
                    )]
                    unsafe {
                        held.release();
                    }
                }
            }
        }
    }

    /// `format` called on the `Date` `from` with `pattern`. It answers the
    /// rendered text, or the sentence it threw.
    fn date_formatted(from: (i16, i8, i8), pattern: &str) -> Result<String, String> {
        let mut ctx = Ctx::buffered();
        let args = [
            date_built(civil::date(from.0, from.1, from.2)),
            Value::str(NvsStr::new(pattern.as_bytes())),
        ];
        let answer = match nvs_runtime::call(nvs_core_time_date_format, &mut ctx, &args) {
            Ok(text) => {
                let read = text
                    .as_text()
                    .expect("`format` answers a `string`")
                    .to_owned();
                #[expect(unsafe_code, reason = "this frame owns the text the member answered")]
                unsafe {
                    text.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        };
        for argument in args {
            #[expect(unsafe_code, reason = "the argument list owns the one reference")]
            unsafe {
                argument.release();
            }
        }
        answer
    }

    /// `format` renders every calendar field a date has, at both ends of the
    /// year range, and throws for every time-of-day or zone letter instead of
    /// inventing a midnight or a UTC. Each refusal names which of the two the
    /// pattern asked for.
    // covers: Core\Time\Date::format
    #[test]
    fn date_format_renders_calendar_fields_and_refuses_a_clock_or_zone_letter() {
        let tuesday = (2024, 3, 5);
        for (pattern, rendered) in [
            ("yyyy-MM-dd", "2024-03-05"),
            ("EEEE, d MMMM yyyy", "Tuesday, 5 March 2024"),
            ("d.M.yy", "5.3.24"),
            ("EEE MMM d", "Tue Mar 5"),
            ("QQQ yyyy", "Q1 2024"),
            ("'day' D", "day 65"),
            ("", ""),
        ] {
            assert_eq!(
                date_formatted(tuesday, pattern),
                Ok(rendered.to_owned()),
                "{pattern:?}"
            );
        }
        assert_eq!(
            date_formatted((-9999, 1, 1), "yyyy-MM-dd"),
            Ok("-9999-01-01".to_owned())
        );
        assert_eq!(
            date_formatted((9999, 12, 31), "yyyy-MM-dd"),
            Ok("9999-12-31".to_owned())
        );
        for pattern in ["HH", "hh", "mm", "ss", "a", "yyyy-MM-dd HH:mm"] {
            let refused = date_formatted(tuesday, pattern).expect_err("a clock letter");
            assert!(
                refused.contains("names no time of day"),
                "{pattern:?}: {refused}"
            );
        }
        for pattern in ["z", "XXX", "yyyy-MM-dd z"] {
            let refused = date_formatted(tuesday, pattern).expect_err("a zone letter");
            assert!(refused.contains("names no zone"), "{pattern:?}: {refused}");
        }
        let refused = date_formatted(tuesday, "'unclosed").expect_err("an open literal");
        assert!(
            refused.starts_with(r"Core\Time\Date::format(): "),
            "{refused}"
        );
    }

    /// `Core\Time\Date::at` called the way a compiled call site calls it. It
    /// answers the `Date` it built, or the sentence it threw.
    fn date_at(year: i64, month: u64, day: u64) -> Result<civil::Date, String> {
        let mut ctx = Ctx::buffered();
        let args = [Value::int(year), Value::uint(month), Value::uint(day)];
        match nvs_runtime::call(nvs_core_time_date_at, &mut ctx, &args) {
            Ok(built) => {
                let read = date_of(&[built], 0, "test").expect("`at` answers a `Date`");
                #[expect(unsafe_code, reason = "this frame owns the `Date` the member answered")]
                unsafe {
                    built.release();
                }
                Ok(read)
            }
            Err(_) => Err(ctx
                .take_pending()
                .expect("a refused call leaves its sentence pending")
                .into_owned()),
        }
    }

    /// `at` builds a date only when all three fields name one that exists.
    /// Every month's last day and the day after it are asserted together, in
    /// a leap year and in a common one, and so are both ends of the year and
    /// month ranges. A number too wide for any field throws one sentence.
    // covers: Core\Time\Date::at
    #[test]
    fn date_at_builds_only_a_date_that_exists() {
        const LEAP: [u64; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for year in [2023_i64, 2024] {
            for (month, last) in (1_u64..).zip(LEAP) {
                let last = if year == 2023 && month == 2 { 28 } else { last };
                let built = date_at(year, month, last).expect("the month's last day");
                assert_eq!(
                    (i64::from(built.year()), built.month(), built.day()),
                    (
                        year,
                        i8::try_from(month).expect("fits"),
                        i8::try_from(last).expect("fits")
                    )
                );
                assert!(
                    date_at(year, month, last + 1).is_err(),
                    "{year}-{month}-{} is not a date",
                    last + 1
                );
                assert!(date_at(year, month, 0).is_err(), "{year}-{month}-0");
            }
        }
        for (year, month, accepted) in [
            (9999, 12, true),
            (10000, 1, false),
            (-9999, 1, true),
            (-10000, 12, false),
            (2024, 1, true),
            (2024, 0, false),
            (2024, 12, true),
            (2024, 13, false),
        ] {
            assert_eq!(
                date_at(year, month, 1).is_ok(),
                accepted,
                "{year}-{month}-1"
            );
        }
        let too_wide = r"Core\Time\Date::at(): a year is within -9999..=9999, and a month and a day are numbers a calendar writes";
        for (year, month, day) in [
            (i64::MIN, 1, 1),
            (i64::MAX, 1, 1),
            (2024, u64::MAX, 1),
            (2024, 1, u64::MAX),
        ] {
            assert_eq!(
                date_at(year, month, day),
                Err(too_wide.to_owned()),
                "{year}-{month}-{day}"
            );
        }
    }
}
