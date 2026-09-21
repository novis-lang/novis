//! The two pieces of CLDR Novis carries, which are one module because they
//! are one body of data:
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 4's date pattern grammar — the one `Core\Time\DateTime::format` emits
//! from and `Core\Time::parse` reads with, in **CLDR** letters
//! (`yyyy-MM-dd HH:mm:ss`, `EEEE, d MMMM yyyy`) rather than PHP's `date()`
//! ones — and `rule:programs/framework-core-half`'s `Core\Cldr::pluralCategory`, the cardinal plural rules a message
//! catalog selects a form with, and `Core\Cldr::ordinalCategory` beside it for
//! the forms a *place* takes.
//!
//! # Why this is written here rather than taken from a crate
//!
//! [`jiff`] already carries a pattern engine, but it is `strftime`'s — a
//! *third* letter grammar, neither the one § 4 wrote nor the one PHP wrote. A
//! full CLDR implementation is `icu`, which is a locale-data dependency an
//! order of magnitude larger than everything else in `Core` put together and
//! which `rule:packaging/a-c-dependency-answers-two-questions`'s second question ("does it carry weight the language
//! does not need?") answers for. So what exists here is the **closed subset**
//! of CLDR field letters § 4's own examples and PHP's `date()` roster between
//! them reach, and every letter outside it is a diagnostic naming itself
//! rather than a silent literal — which is CLDR's own rule for an unquoted
//! letter, and the opposite of `strftime`'s.
//!
//! # No locale, by decision
//!
//! A month or weekday name renders in CLDR's **root** locale — English — and
//! nothing selects another. That is the same closed door
//! `rule:core-api/tier-placement` shuts on
//! `setlocale`: a process-wide setting that silently changes what a later
//! `format` answers is exactly the ambient state § 4 removed the default
//! timezone for. A program that wants a localized month name has the number,
//! and localization is an application concern rather than a Tier 0 one.
//!
//! # The subset
//!
//! | Letter | Meaning | Counts |
//! |---|---|---|
//! | `G` | era | 1–3 `AD`/`BC`, `GGGG` full, `GGGGG` narrow — read off the year's sign |
//! | `y` | year | `yy` is two digits, any other count zero-pads |
//! | `u` | extended year | the same year, and the only letter that reads a sign back |
//! | `Q` | quarter | 1–2 numeric, `QQQ` is `Q1`, `QQQQ` full, `QQQQQ` narrow |
//! | `M` | month | 1–2 numeric, `MMM` short name, `MMMM` full, `MMMMM` narrow |
//! | `L` | standalone month | `M`'s counts; the root locale spells both alike |
//! | `d` | day of month | numeric, zero-padded to the count |
//! | `D` | day of year | numeric |
//! | `w` | week of year | ISO 8601's numbering, zero-padded to the count |
//! | `W` | week of month | the same rule inside a month, and `0` for a short leading week |
//! | `F` | weekday ordinal in month | numeric — `1` for the first Friday of the month |
//! | `E` | weekday name | 1–3 short, `EEEE` full, `EEEEE` narrow, `EEEEEE` two-letter |
//! | `c` | standalone weekday | `c` is the day's number, 3–6 are `E`'s names |
//! | `a` | AM/PM | any count |
//! | `h` | hour 1–12 | numeric |
//! | `H` | hour 0–23 | numeric |
//! | `K` | hour 0–11 | numeric |
//! | `k` | hour 1–24 | numeric |
//! | `m` | minute | numeric |
//! | `s` | second | numeric |
//! | `S` | fractional second | one digit per count, truncated |
//! | `X` | ISO offset, `Z` at zero | `X` = `±HH[mm]`, `XX` = `±HHmm`, `XXX` = `±HH:MM` |
//! | `x` | ISO offset, never `Z` | same three counts |
//! | `VV` | the zone's IANA identifier | `VV` only |
//! | `A` | milliseconds in the day | numeric, zero-padded to the count |
//! | `g` | modified Julian day | numeric, and signed before `1858-11-17` |
//! | `U` | cyclic year name | the Gregorian calendar has none, so `y`'s rendering |
//! | `r` | related Gregorian year | `u`'s rendering, the same signed year |
//! | `b` | AM/PM, and `noon`/`midnight` at those two instants | 1–4 the name, 5 narrow |
//! | `B` | the day period as a range — `in the morning` | 1–4 the name, 5 narrow |
//! | `z` | the zone's specific name | short below `zzzz`, long at it |
//! | `v` | the zone's generic name | `v` short, `vvvv` long |
//! | `O` | localized GMT | `O` = `GMT-8`, `OOOO` = `GMT-08:00` |
//! | `Z` | ISO offset, basic | 1–3 `±HHmm`, `ZZZZ` localized GMT, `ZZZZZ` = `XXX` |
//! | `Y` | the year `w`'s week belongs to | `y`'s counts, two digits at `YY` |
//! | `e` | local weekday number | `c`'s counts, and its numbering |
//! | `q` | standalone quarter | `Q`'s counts; the root locale spells both alike |
//!
//! A `'…'` run is a literal, and `''` is one apostrophe — CLDR's own quoting.
//!
//! **`y` is the signed proleptic year here, and CLDR's year-of-era is not
//! carried.** CLDR distinguishes `y` (the year *within* an era, always
//! positive, needing a `G` to be unambiguous) from `u` (the signed proleptic
//! year); this subset writes the proleptic year for both, so `y` on a date
//! before year 1 renders `-0043` rather than `0044`. That keeps
//! `format("yyyy-MM-dd")` sortable and reversible over the whole
//! `-9999..=9999` range a `Core\Time\Date` accepts, which is what every caller
//! of it in this repository depends on, and it makes `G` a *rendering* of the
//! sign rather than a second half of the year. The two letters still differ on
//! the way back in: `u` reads a leading `-` and `y` reads digits only, so a
//! date before year 1 round-trips through `u` alone.
//!
//! **The week rule is ISO 8601's, and `w`, `W`, `Y`, `e` and `c` all read
//! it**: a week starts on Monday, and week 1 is the first one with at least
//! four days in the period. CLDR states that rule per *territory* rather than
//! per language, which is exactly the locale data the section above refuses to
//! carry — so one rule is written here, it is the one the majority of that
//! table names, and a `W` whose month opens with a short partial week answers
//! `0`, as ICU's own week-of-month does.
//!
//! **One rule, rather than a table of them, because nothing here selects a
//! territory.** A pattern names no locale and `format` takes none, so a
//! transcribed `weekData` table would be read at one row and no other — and
//! the row it would be read at, CLDR's `001`, counts a week from Monday but
//! calls the first partial one week 1, which is the rule this module did not
//! choose. Carrying both would put `w` and `Y` on different calendars inside
//! one pattern, which is the one thing a week-based year beside its week may
//! not do. So the five letters agree by construction, and a program that wants
//! another territory's numbering has the date and computes it.
//!
//! **No zone *name* is carried, so `z` and `v` render the localized GMT
//! format.** The names `z` asks for — `PDT`, `Pacific Daylight Time` — and the
//! generic ones `v` asks for are per-zone per-locale data of the size the
//! section above refuses, so this subset carries none of it and every one of
//! these letters takes the fallback ICU takes when a locale has none: the
//! short localized GMT format (`GMT-8`) below `zzzz` and at `v`, the long one
//! (`GMT-08:00`) at `zzzz` and `vvvv`, and `GMT` at a zero offset either way.
//! That leaves `O` and `Z` to name the same two spellings outright, which is
//! what they are for. So a pattern's zone letter chooses its *spelling* here
//! and never whether the answer is a name, and an offset is written to the
//! minute, as `X` and `x` already write it.
//!
//! **`b` and `B` read English's day periods**, for the reason above: the
//! periods a locale divides its clock into are locale data, and the one locale
//! this module renders in is the one its month and weekday names are in. So
//! `b` names `midnight` and `noon` at the two instants that have their own
//! name and `AM`/`PM` everywhere else, `B` names the four ranges English
//! divides the rest of the day into, and the instant is decided from the hour,
//! minute and second — the fraction is not read, which is where ICU draws the
//! same line.
//!
//! # The plural rules, and why they are a closed roster
//!
//! `rule:programs/framework-core-half`'s row reads "one member exposing the CLDR data
//! `nvs_stdlib::cldr` already holds". That was never true of this module: what
//! it held was the pattern grammar above and no plural data at all, so the row
//! is a member *and* the table behind it. The row's reason survives unchanged —
//! § 3's `Web\I18n` formats messages over this rather than shipping a second
//! copy of CLDR — and it is why the member is `Core` at all.
//!
//! What is carried is CLDR's **cardinal** rules, as [`RuleSet`]'s arms and the
//! language-subtag table [`RULES`] maps onto them. Ordinal rules (`1st`,
//! `2nd`) are a second table, [`ORDINALS`], and the section on them below owns
//! what it carries and what it refuses.
//!
//! **A language whose rules are not carried throws rather than falling back.**
//! There is an obvious cheaper design — answer English's `one`/`other` for
//! anything unrecognized — and it is the wrong one twice over: it is
//! `rule:errors/ambiguous-input-refused`'s
//! repair-instead-of-refuse, and it is silently wrong in the direction that
//! matters, since a Russian catalog written against `one`/`other` reads
//! correctly for 1 and wrongly for 2, 5 and 11 alike. The refusal names the
//! subtag, so the gap is a message rather than a mistranslation. Widening the
//! roster is data: one arm if the rule shape is new, otherwise one row in
//! [`RULES`].
//!
//! **[`RULES`] is a roster rather than all of CLDR, and it names no absence.**
//! What is in it is what has been transcribed, and the line is drawn where a
//! catalog would notice a hole rather than where transcription happened to
//! stop: `da`, `fil`, `tl` and `ceb` sit in it beside `be`, `he`, `mt` and the
//! Sorbian pair because a roster that answers for `af` and refuses Danish has
//! a hole rather than an edge.
//! `every_language_named_absent_in_the_gap_note_now_has_a_rule` is what holds
//! this paragraph to the table.
//!
//! **The operands come from what the count shows, which is why `decimal` is
//! the exact one.** CLDR's `v` and `f` are the *visible* fraction digits, so
//! English puts `1` in `One` and `1.0` in `Other`. A `decimal` carries its
//! scale (`rule:types/decimal`) and so
//! answers that distinction exactly; an `int` has no fraction; a `float` has no
//! scale, so its digits are read off the shortest representation that
//! round-trips — which is what `echo` writes for the same value, and therefore
//! is what a reader sees.
//!
//! CLDR's `e` operand — the compact-decimal exponent behind `1,2M` — is 0
//! everywhere here, because Novis has no compact notation to produce one. The
//! `many` arms that read it in the published rules are written at `e = 0`,
//! which is the millions rule `ca`, `es`, `fr`, `it` and `pt` share.
//!
//! # The ordinal rules, and the one place they answer where the cardinal ones
//! refuse
//!
//! CLDR's ordinal rules are a second table — the forms `1st`, `2nd`, `3rd`,
//! `4th` take, rather than the forms `1 file`/`2 files` take — and
//! the member [`ORDINAL_MEMBER`] names reads it. They answer with the same six
//! categories, so no second enum is registered: `Two` is Welsh's `2il` here and
//! Welsh's two-thing form there, and which one a program meant is which member
//! it called.
//!
//! **[`ORDINALS`] holds only the languages that mark a form**, and a language
//! [`RULES`] carries but this table does not answers `Other`. That is a
//! deliberate difference from the cardinal member's refusal, and the reason is
//! that the two absences are not the same fact. A cardinal rule CLDR does not
//! publish is unknown, and guessing one mistranslates a count the caller can
//! name; an ordinal rule it does not publish is *published as nothing* — CLDR's
//! own ordinal data files every language with no marked form into one bucket,
//! and `Other` is the unmarked form that bucket names. So the roster is the
//! languages that differ from it, which is how CLDR writes the same data.
//! A language neither table carries is still refused, by [`rules_for`], which
//! is the boundary both members share.
//!
//! **That default is never an omission, because the roster is closed over
//! [`RULES`].** Every language CLDR publishes a marked ordinal rule for and the
//! cardinal roster carries has a row here, so a language answering `Other` is a
//! language CLDR publishes nothing marked for — and one CLDR marks that the
//! cardinal roster does not carry is refused by both members rather than
//! answered, since the ordinal lookup falls through to [`rules_for`]. The only
//! way this table can fall behind CLDR is the cardinal roster growing without
//! it, which is what
//! `every_language_cldr_gives_an_ordinal_rule_is_on_the_ordinal_roster`
//! carries CLDR's own ordinal groups to catch.
//!
//! # A written pattern is compiled once per core, an assembled one per call
//!
//! `rule:expressions/intrinsic-literals` makes `Core\Time\DateTime::format` and
//! `Core\Time::parse` intrinsics: a pattern written as a literal is compiled by
//! [`validate`] *while checking*, so a malformed one is a diagnostic pointing
//! at the letter rather than the throw [`compile`] answers an assembled one
//! with, and the call site carries [`PREPARED_PATTERN`] as
//! `crate::registry::PREPARED_MEMBERS`' argument 0. That word is what admits
//! the text to [`compiled`]'s per-core cache.
//!
//! **A pattern the program assembled is never keyed there**, which is the whole
//! reason the word is consulted rather than the text alone: what a program
//! writes is a closed set fixed when it was compiled, while what a request
//! assembles is not, so caching the second would make a core's footprint a
//! function of the text requests send it. The bound is therefore O(written
//! patterns per core) rather than O(requests served), and [`CACHE_CAPACITY`]
//! caps even that — cleared wholesale when it fills, for the reason
//! [`crate::regex`]'s own cache states at length.
//!
//! **What it spends:** one compiled pattern per distinct written pattern per
//! core — a `Vec` of [`Piece`], one entry per field letter and literal run —
//! held for the life of the core rather than of the request, and never more
//! than [`CACHE_CAPACITY`] of them. That those bytes outlive the request that
//! paid for them, what the request that clears the cache is credited with in
//! exchange, and why no accounting bracket takes a store shaped this way, are
//! [`crate::regex`]'s § *What a compiled pattern costs, and where it is held* —
//! one question about both caches, answered in one place.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::RangeInclusive;
use std::rc::Rc;

use jiff::Zoned;
use jiff::civil;
use jiff::tz::{Offset, TimeZone};
use nvs_runtime::{Fault, ThrownClass, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// One field a pattern letter names, already resolved from the letter so
/// nothing downstream matches on a `u8` a second time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    /// `y`
    Year,
    /// `M`
    Month,
    /// `d`
    Day,
    /// `D`
    DayOfYear,
    /// `E`
    Weekday,
    /// `a`
    AmPm,
    /// `h` — 1–12.
    Hour12,
    /// `H` — 0–23.
    Hour23,
    /// `K` — 0–11.
    Hour11,
    /// `k` — 1–24.
    Hour24,
    /// `m`
    Minute,
    /// `s`
    Second,
    /// `S`
    Fraction,
    /// `X` — `Z` at a zero offset.
    OffsetZ,
    /// `x` — a signed offset even at zero.
    Offset,
    /// `VV`
    ZoneId,
    /// `G` — AD or BC.
    Era,
    /// `u` — the same year [`Self::Year`] renders, and the one letter that
    /// reads its sign back.
    ExtendedYear,
    /// `Q`
    Quarter,
    /// `w` — ISO 8601's week of the year.
    WeekOfYear,
    /// `W` — the same rule inside one month.
    WeekOfMonth,
    /// `L` — the standalone month, which the root locale spells as `M` does.
    StandaloneMonth,
    /// `c` — the standalone weekday, whose count 1 is a number where `E`'s is
    /// a name.
    StandaloneWeekday,
    /// `F` — which weekday of the month this is: 1 for the first Friday.
    DayOfWeekInMonth,
    /// `A` — milliseconds since local midnight.
    MillisInDay,
    /// `g` — the modified Julian day, whose day starts at local midnight
    /// rather than at the astronomical noon.
    JulianDay,
    /// `b` — [`Self::AmPm`] with `noon` and `midnight` at the two instants
    /// that have a name of their own.
    DayPeriod,
    /// `B` — the period a locale names a *range* of the clock with:
    /// `in the morning`.
    FlexibleDayPeriod,
    /// `z` — the zone's specific name.
    ZoneSpecific,
    /// `v` — the zone's generic name.
    ZoneGeneric,
    /// `Z` — the ISO 8601 basic offset, whose two long counts are the
    /// spellings [`Self::OffsetGmt`] and [`Self::OffsetZ`] carry.
    OffsetBasic,
    /// `O` — the localized GMT format, short at `O` and long at `OOOO`.
    OffsetGmt,
    /// `Y` — the year [`Self::WeekOfYear`]'s week belongs to, which is the one
    /// before it for a week straddling January.
    WeekBasedYear,
    /// `e` — the weekday counted from the day the week starts on, which is
    /// what [`Self::StandaloneWeekday`]'s count 1 already answers.
    LocalWeekday,
}

impl Field {
    /// The field a CLDR pattern letter names, or `None` for a letter no
    /// pattern may carry.
    ///
    /// CLDR reserves **every** ASCII letter, so what is left after the table
    /// above is the skeleton letters `j`, `J` and `C` — which name a
    /// preference between `h` and `H` rather than a field, and are defined for
    /// a requested skeleton rather than for a pattern — deprecated `l`, and
    /// the letters CLDR gives no meaning at all. [`compile`] refuses each by
    /// name, which is CLDR's own rule for a reserved letter.
    fn of(letter: u8) -> Option<Self> {
        Some(match letter {
            b'y' => Self::Year,
            b'M' => Self::Month,
            b'd' => Self::Day,
            b'D' => Self::DayOfYear,
            b'E' => Self::Weekday,
            b'a' => Self::AmPm,
            b'h' => Self::Hour12,
            b'H' => Self::Hour23,
            b'K' => Self::Hour11,
            b'k' => Self::Hour24,
            b'm' => Self::Minute,
            b's' => Self::Second,
            b'S' => Self::Fraction,
            b'X' => Self::OffsetZ,
            b'x' => Self::Offset,
            b'V' => Self::ZoneId,
            b'G' => Self::Era,
            b'u' => Self::ExtendedYear,
            b'Q' => Self::Quarter,
            b'w' => Self::WeekOfYear,
            b'W' => Self::WeekOfMonth,
            b'L' => Self::StandaloneMonth,
            b'c' => Self::StandaloneWeekday,
            b'F' => Self::DayOfWeekInMonth,
            // The two year letters a calendar other than the Gregorian one
            // would separate: `U` is the cyclic year name, which a calendar
            // without one renders as the numeric year, and `r` is the related
            // Gregorian year, which under the Gregorian calendar is the year
            // itself. So each resolves to the letter it agrees with here, and
            // the two-digit window `yy` has is the whole of what separates
            // them — `rr` writes the year, as `uu` does.
            b'U' => Self::Year,
            b'r' => Self::ExtendedYear,
            b'A' => Self::MillisInDay,
            b'g' => Self::JulianDay,
            b'b' => Self::DayPeriod,
            b'B' => Self::FlexibleDayPeriod,
            b'z' => Self::ZoneSpecific,
            b'v' => Self::ZoneGeneric,
            b'Z' => Self::OffsetBasic,
            b'O' => Self::OffsetGmt,
            b'Y' => Self::WeekBasedYear,
            b'e' => Self::LocalWeekday,
            // The standalone quarter, which the root locale spells as `Q`
            // does and gives the same counts — so, like `U` and `r` above, it
            // resolves onto the letter it agrees with rather than carrying a
            // variant that would render the same bytes.
            b'q' => Self::Quarter,
            _ => return None,
        })
    }

    /// The sentence refusing `count` repetitions of this field's letter, for
    /// the letters CLDR gives a closed set of counts rather than a padding
    /// width.
    ///
    /// A count outside that set is refused rather than rendered as the
    /// nearest one, which is [`compile`]'s rule for an unknown letter applied
    /// to a known one: ICU emits nothing at all for these, and an empty
    /// rendering is the silent answer this grammar does not give.
    fn count_refused(self, count: usize) -> Option<&'static str> {
        match self {
            Self::ZoneId if count != 2 => {
                Some("`V` names a zone only as `VV`, the IANA identifier")
            }
            Self::ZoneGeneric if count != 1 && count != 4 => {
                Some("`v` names a zone as `v` or `vvvv`, the short and long generic forms")
            }
            Self::OffsetGmt if count != 1 && count != 4 => Some(
                "`O` names an offset as `O` or `OOOO`, the short and long localized GMT formats",
            ),
            Self::OffsetBasic if count > 5 => {
                Some("`Z` names an offset as `Z` through `ZZZZZ` and no further")
            }
            _ => None,
        }
    }

    /// Whether this field says something about the **zone** rather than about
    /// a civil field — which is what makes it legal in a `format` pattern and
    /// refused in a `parse` one, since `Core\Time::parse` takes the zone as
    /// its own third argument.
    fn is_zonal(self) -> bool {
        matches!(
            self,
            Self::OffsetZ
                | Self::Offset
                | Self::ZoneId
                | Self::ZoneSpecific
                | Self::ZoneGeneric
                | Self::OffsetBasic
                | Self::OffsetGmt
        )
    }

    /// Whether this field says something about the **time of day** — the half
    /// of a civil datetime a zone-free `Core\Time\Date` does not carry, and so
    /// the half [`date_fields_only`] refuses.
    fn is_time_of_day(self) -> bool {
        matches!(
            self,
            Self::AmPm
                | Self::Hour12
                | Self::Hour23
                | Self::Hour11
                | Self::Hour24
                | Self::Minute
                | Self::Second
                | Self::Fraction
                | Self::MillisInDay
                | Self::DayPeriod
                | Self::FlexibleDayPeriod
        )
    }

    /// Whether this field says something about the **calendar** — the other
    /// half of the same split, and the one [`time_fields_only`] refuses.
    ///
    /// Written as the complement rather than as a third roster: the three
    /// predicates partition the letters, and a roster here is one a letter
    /// added to the table above can be left out of silently — which is a
    /// `Core\Time\TimeOfDay` rendering a year it does not have.
    fn is_calendar(self) -> bool {
        !self.is_time_of_day() && !self.is_zonal()
    }
}

/// One compiled piece of a pattern: text to emit verbatim, or a field to
/// render.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Piece {
    /// A quoted run, or any non-letter character.
    Literal(String),
    /// A run of one repeated letter, with its length — CLDR's count, which is
    /// what picks between `MM` and `MMMM`.
    Field(Field, usize),
}

/// English root-locale month names, full then short then narrow, indexed by
/// month − 1.
const MONTHS: [(&str, &str, &str); 12] = [
    ("January", "Jan", "J"),
    ("February", "Feb", "F"),
    ("March", "Mar", "M"),
    ("April", "Apr", "A"),
    ("May", "May", "M"),
    ("June", "Jun", "J"),
    ("July", "Jul", "J"),
    ("August", "Aug", "A"),
    ("September", "Sep", "S"),
    ("October", "Oct", "O"),
    ("November", "Nov", "N"),
    ("December", "Dec", "D"),
];

/// English root-locale era names, full then short then narrow, indexed by
/// [`Field::Era`]'s own order: AD first, because a proleptic year above zero is
/// the ordinary case.
const ERAS: [(&str, &str, &str); 2] = [("Anno Domini", "AD", "A"), ("Before Christ", "BC", "B")];

/// English root-locale quarter names, indexed by quarter − 1. Only `QQQQ`
/// spells one out; the shorter counts are `Q1` and the bare number.
const QUARTERS: [&str; 4] = ["1st quarter", "2nd quarter", "3rd quarter", "4th quarter"];

/// English root-locale weekday names, full then short then narrow then
/// two-letter, indexed Monday-first.
const WEEKDAYS: [(&str, &str, &str, &str); 7] = [
    ("Monday", "Mon", "M", "Mo"),
    ("Tuesday", "Tue", "T", "Tu"),
    ("Wednesday", "Wed", "W", "We"),
    ("Thursday", "Thu", "T", "Th"),
    ("Friday", "Fri", "F", "Fr"),
    ("Saturday", "Sat", "S", "Sa"),
    ("Sunday", "Sun", "S", "Su"),
];

/// One day period: what it is called, what its narrow name is, the hour it
/// fixes if it names an instant rather than a range, and which half of the
/// clock its **midpoint** falls in.
///
/// The midpoint is what a 12-hour field beside it reads, which is CLDR's own
/// rule for a period spanning both halves: `at night` runs from 21:00 to
/// 06:00, and its midpoint is 01:30, so an `h` beside it is a morning hour.
#[derive(Clone, Copy)]
struct DayPeriod {
    name: &'static str,
    narrow: &'static str,
    hour: Option<i8>,
    afternoon: bool,
}

/// The four periods `b` names.
const AM: DayPeriod = DayPeriod {
    name: "AM",
    narrow: "a",
    hour: None,
    afternoon: false,
};
const PM: DayPeriod = DayPeriod {
    name: "PM",
    narrow: "p",
    hour: None,
    afternoon: true,
};
const MIDNIGHT: DayPeriod = DayPeriod {
    name: "midnight",
    narrow: "mi",
    hour: Some(0),
    afternoon: false,
};
const NOON: DayPeriod = DayPeriod {
    name: "noon",
    narrow: "n",
    hour: Some(12),
    afternoon: true,
};

/// The four ranges `B` divides the rest of the clock into, whose narrow name
/// is their wide one in this locale.
const MORNING: DayPeriod = DayPeriod {
    name: "in the morning",
    narrow: "in the morning",
    hour: None,
    afternoon: false,
};
const AFTERNOON: DayPeriod = DayPeriod {
    name: "in the afternoon",
    narrow: "in the afternoon",
    hour: None,
    afternoon: true,
};
const EVENING: DayPeriod = DayPeriod {
    name: "in the evening",
    narrow: "in the evening",
    hour: None,
    afternoon: true,
};
const NIGHT: DayPeriod = DayPeriod {
    name: "at night",
    narrow: "at night",
    hour: None,
    afternoon: false,
};

/// `b`'s roster, for [`read_field`] to match text against.
const DAY_PERIODS: [DayPeriod; 4] = [AM, PM, MIDNIGHT, NOON];

/// `B`'s roster, which shares the two instants with `b`'s and names ranges
/// where that one names halves.
const FLEXIBLE_DAY_PERIODS: [DayPeriod; 6] = [MIDNIGHT, NOON, MORNING, AFTERNOON, EVENING, NIGHT];

/// The day period this clock time falls in, for `B` when `flexible` and for
/// `b` otherwise.
///
/// Midnight and noon are the two instants with a name of their own, and the
/// hour, minute and second decide whether the clock is at one — the fraction
/// is not read, which is ICU's own boundary and makes `12:00:00.001` noon.
fn day_period(at: civil::Time, flexible: bool) -> DayPeriod {
    let hour = at.hour();
    if at.minute() == 0 && at.second() == 0 {
        if hour == 0 {
            return MIDNIGHT;
        }
        if hour == 12 {
            return NOON;
        }
    }
    if !flexible {
        return if hour < 12 { AM } else { PM };
    }
    match hour {
        6..=11 => MORNING,
        12..=17 => AFTERNOON,
        18..=20 => EVENING,
        _ => NIGHT,
    }
}

/// The day [`Field::JulianDay`] counts from: the modified Julian day is the
/// astronomical one less 2400000.5, so its zero is this date's local midnight.
const JULIAN_EPOCH: civil::Date = civil::Date::constant(1858, 11, 17);

/// The modified Julian day of `date`.
fn julian_day(date: civil::Date) -> i32 {
    date.since((jiff::Unit::Day, JULIAN_EPOCH))
        .expect("a civil date is a whole number of days from 1858")
        .get_days()
}

/// CLDR's localized GMT format, which is what a zone whose name this subset
/// does not carry renders as: `GMT` at a zero offset, `GMT-08:00` long and
/// `GMT-8` short, with the minutes written only when the short form has any.
///
/// The offset is written to the minute, as `X` and `x` here already write it.
fn localized_gmt(out: &mut String, offset: Offset, long: bool) {
    let seconds = offset.seconds();
    out.push_str("GMT");
    if seconds == 0 {
        return;
    }
    out.push(if seconds < 0 { '-' } else { '+' });
    let total = seconds.unsigned_abs();
    let (hours, minutes) = (u64::from(total / 3_600), u64::from((total % 3_600) / 60));
    pad(out, hours, if long { 2 } else { 1 });
    if long || minutes != 0 {
        out.push(':');
        pad(out, minutes, 2);
    }
}

/// The word `crate::registry::PREPARED_MEMBERS`' argument 0 carries when the
/// pattern beside it is a literal this build's checker already compiled.
///
/// A word of its own rather than a reuse of [`crate::regex`]'s first tier: one
/// slot is shared by members reading different grammars, so a word minted for
/// another one decodes here as nothing prepared — [`prepared`]'s equality —
/// instead of as this module's fact.
pub const PREPARED_PATTERN: i64 = 3;

/// The absence, which every member on that roster spells the same way and
/// [`crate::regex::PREPARED_NONE`] documents in full.
pub use crate::regex::PREPARED_NONE;

/// Whether `code` says the pattern beside it is one the compiler prepared.
///
/// A word this build does not know is read as "nothing was prepared" rather
/// than refused, for the reason [`crate::regex::prepared_tier`] gives: a
/// compiler-version skew then costs a compile, never a request.
#[must_use]
pub fn prepared(code: i64) -> bool {
    code == PREPARED_PATTERN
}

/// [`prepared`] over the argument a helper holds rather than the word inside
/// it — the spelling both members on that roster reach for.
pub(crate) fn prepared_arg(word: &Value) -> bool {
    prepared(word.as_int().unwrap_or(PREPARED_NONE))
}

/// How many compiled patterns one core holds before the cache is cleared —
/// this module's own docs own the reasoning and what it spends.
const CACHE_CAPACITY: usize = 64;

thread_local! {
    /// This core's compiled patterns, keyed by the pattern text alone: a
    /// pattern compiles to one piece list and to nothing else, so there is no
    /// second half of a key the way [`crate::regex`]'s flags and budget are.
    /// Only a pattern the compiler prepared is ever keyed here, which is the
    /// module docs' bound — a request cannot mint an entry.
    static CACHE: RefCell<Vec<(String, Rc<Vec<Piece>>)>> = const { RefCell::new(Vec::new()) };
}

/// [`compile`], answered from this core's cache when the call site was handed
/// [`PREPARED_PATTERN`], and compiled at the call when it was not.
///
/// # Errors
///
/// [`compile`]'s, unchanged. A prepared pattern is one the checker already
/// accepted, so the refusal is reachable only on the path that prepared
/// nothing — and it is still made there rather than assumed away.
pub(crate) fn compiled(pattern: &str, prepared: bool) -> Result<Rc<Vec<Piece>>, String> {
    if !prepared {
        return compile(pattern).map(Rc::new);
    }
    if let Some(hit) = CACHE.with_borrow(|cache| {
        cache
            .iter()
            .find(|(key, _)| key == pattern)
            .map(|(_, pieces)| Rc::clone(pieces))
    }) {
        return Ok(hit);
    }
    let built = Rc::new(compile(pattern)?);
    CACHE.with_borrow_mut(|cache| {
        if cache.len() >= CACHE_CAPACITY {
            cache.clear();
        }
        cache.push((pattern.to_owned(), Rc::clone(&built)));
    });
    Ok(built)
}

/// Compiles `pattern` into the pieces [`render`] and [`read`] walk, or the
/// sentence a caller throws for a pattern this subset does not carry.
///
/// # Errors
///
/// A one-sentence reason naming the offending letter or the unterminated
/// quote — never a position, since a pattern is short and the letter is what
/// a reader fixes.
pub(crate) fn compile(pattern: &str) -> Result<Vec<Piece>, String> {
    let bytes = pattern.as_bytes();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'\'' {
            at += 1;
            // `''` outside a quoted run is one apostrophe, which is the same
            // rule as inside one, so both reach here.
            if at < bytes.len() && bytes[at] == b'\'' {
                push_literal(&mut pieces, "'");
                at += 1;
                continue;
            }
            let mut literal = String::new();
            loop {
                let Some(&byte) = bytes.get(at) else {
                    return Err("a `'` opens a literal that nothing closes".to_owned());
                };
                at += 1;
                if byte != b'\'' {
                    literal.push(char::from(byte));
                    continue;
                }
                if bytes.get(at) == Some(&b'\'') {
                    literal.push('\'');
                    at += 1;
                    continue;
                }
                break;
            }
            push_literal(&mut pieces, &literal);
            continue;
        }
        if !byte.is_ascii_alphabetic() {
            // A multi-byte character is copied whole: only ASCII letters are
            // pattern letters, so anything else is text and its continuation
            // bytes are never alphabetic.
            let start = at;
            at += 1;
            while at < bytes.len() && !bytes[at].is_ascii() {
                at += 1;
            }
            push_literal(&mut pieces, &pattern[start..at]);
            continue;
        }
        let mut count = 0;
        while at < bytes.len() && bytes[at] == byte {
            count += 1;
            at += 1;
        }
        let field = Field::of(byte).ok_or_else(|| {
            format!(
                "`{}` is not a pattern letter this library carries; quote it as `'{}'` to emit it \
                 literally",
                char::from(byte),
                char::from(byte)
            )
        })?;
        if let Some(reason) = field.count_refused(count) {
            return Err(reason.to_owned());
        }
        pieces.push(Piece::Field(field, count));
    }
    Ok(pieces)
}

/// Whether `pattern` is one [`compile`] can read, for a caller that wants the
/// refusal and not the pieces —
/// `rule:expressions/intrinsic-literals`'s fold,
/// which reads a *literal* pattern while checking and reports § 3's diagnostic
/// instead of the throw the runtime would have made.
///
/// # Errors
///
/// [`compile`]'s own sentence, unchanged: the fold and the runtime path are
/// one implementation, so a pattern refused here is exactly one the first call
/// would have thrown on (§ 4). This is what `nvs-types` reaches rather than
/// [`compile`] itself, because a caller that discards the pieces should not
/// make `Piece` and `Field` public API — the same split
/// [`crate::format::placeholders`] makes beside the renderer it shares a walk
/// with.
///
/// The *member* restrictions are not applied here: they are a rule about the
/// member rather than about the grammar, so a caller that owes one asks for it
/// by name — [`validate_civil`] is `Core\Time::parse`'s. That keeps this
/// answer sound for everyone else: everything it refuses, every caller refuses.
pub fn validate(pattern: &str) -> Result<(), String> {
    compile(pattern).map(|_| ())
}

/// [`validate`] plus `Core\Time::parse`'s own restriction: a pattern the
/// grammar reads, naming no zone.
///
/// The member half of `rule:expressions/intrinsic-list-is-closed`'s roster,
/// which addresses a row's restriction as well as its grammar. One walk
/// answers both, so the diagnostic a checking run reports is the first refusal
/// the first call would have thrown, in the order [`crate::time`]'s own reader
/// makes them.
///
/// # Errors
///
/// [`compile`]'s sentence where the pattern is not one, and
/// [`civil_fields_only`]'s where it names a zone.
pub fn validate_civil(pattern: &str) -> Result<(), String> {
    civil_fields_only(&compile(pattern)?)
}

/// Appends `text` to the pieces, merging it into a trailing literal so that
/// `"-"`-separated runs are one piece rather than three.
fn push_literal(pieces: &mut Vec<Piece>, text: &str) {
    match pieces.last_mut() {
        Some(Piece::Literal(held)) => held.push_str(text),
        _ => pieces.push(Piece::Literal(text.to_owned())),
    }
}

/// Refuses a pattern that names anything a civil **date** does not carry: a
/// time of day, or a zone.
///
/// `Core\Time\Date::format` is the one caller, and it refuses rather than
/// substitutes for [`read`]'s reason in the other direction. Rendering a date
/// through a pattern with an `HH` in it would have to invent a time of day,
/// and every value it could invent — midnight, the system's own clock — is a
/// wrong answer stated confidently. A `Date` that wants one names the time it
/// means, which is what `DateTime` is.
///
/// # Errors
///
/// A one-sentence reason, in [`compile`]'s shape.
pub(crate) fn date_fields_only(pieces: &[Piece]) -> Result<(), String> {
    for piece in pieces {
        let Piece::Field(field, _) = piece else {
            continue;
        };
        if field.is_time_of_day() {
            return Err(
                "a pattern rendering a date names no time of day — a `Core\\Time\\Date` carries \
                 none"
                    .to_owned(),
            );
        }
        if field.is_zonal() {
            return Err(
                "a pattern rendering a date names no zone — a `Core\\Time\\Date` is zone-free"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

/// Refuses a pattern that names anything a `Core\Time\TimeOfDay` does not
/// carry: a calendar field, or a zone.
///
/// [`date_fields_only`]'s other half, and it exists for the same reason —
/// a `yyyy` rendered from a time of day would have to invent a year, and an
/// invented one is a wrong answer stated confidently.
///
/// # Errors
///
/// A one-sentence reason, in [`compile`]'s shape.
pub(crate) fn time_fields_only(pieces: &[Piece]) -> Result<(), String> {
    for piece in pieces {
        let Piece::Field(field, _) = piece else {
            continue;
        };
        if field.is_calendar() {
            return Err(
                "a pattern rendering a time of day names no calendar field — a \
                 `Core\\Time\\TimeOfDay` carries none"
                    .to_owned(),
            );
        }
        if field.is_zonal() {
            return Err(
                "a pattern rendering a time of day names no zone — a `Core\\Time\\TimeOfDay` is \
                 zone-free"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

/// Refuses a pattern that names a **zone**, which is the half of the same
/// split [`read`] makes rather than [`render`].
///
/// `Core\Time::parse` is the one caller, and the refusal is about the
/// *pattern* rather than about the text: that member takes a `Zone` as its own
/// third argument, so a pattern naming one too would give two answers with no
/// rule to pick between them. Making it here rather than inside [`read`] is
/// what decides which spec § 10 class it is — a pattern this call site wrote
/// is a bug in the program (`LogicError`), where text that does not match a
/// well-formed pattern is the input's failure (`ParseError`), and a refusal
/// raised while walking the text would have been the second where it is the
/// first. Nothing about the civil fields themselves is refused, which is the
/// half that makes this the third sibling rather than a stricter one: a parse
/// pattern may name any of them, and one it does not name is left at the start
/// of its range.
///
/// # Errors
///
/// A one-sentence reason, in [`compile`]'s shape.
pub(crate) fn civil_fields_only(pieces: &[Piece]) -> Result<(), String> {
    for piece in pieces {
        let Piece::Field(field, _) = piece else {
            continue;
        };
        if field.is_zonal() {
            return Err(
                "a pattern that parses names no zone — `Core\\Time::parse` takes one as its third \
                 argument"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

/// Renders `at` through `pieces`.
///
/// Total: every field reads a component the value already has, so there is
/// nothing here that can fail once [`compile`] accepted the pattern.
pub(crate) fn render(pieces: &[Piece], at: &Zoned) -> String {
    render_placed(
        pieces,
        at.datetime(),
        at.offset(),
        at.time_zone().iana_name().unwrap_or("UTC"),
    )
}

/// [`render`] for a value that names no instant — a `Core\Time\Date` or a
/// `Core\Time\TimeOfDay`, each of which carries one civil half and nothing
/// else.
///
/// It exists because placing such a value on the timeline first is **not
/// total**: `jiff`'s instant range is narrower than its civil one at both
/// ends, so `Core\Time\Date::at(-9999, 1, 1)` and `::at(9999, 12, 31)` — both
/// inside the year range that member accepts — have no midnight UTC to be
/// converted to, and rendering either one used to abort the request with a
/// `FATAL` that no program could catch.
///
/// The offset and the zone name are never read through this entry point —
/// both callers run their pattern through [`date_fields_only`] or
/// [`time_fields_only`] first, and each of those refuses a zonal field — so
/// UTC is passed for them because UTC is the placement both members already
/// used, and every pattern that rendered before renders the same bytes now.
pub(crate) fn render_utc(pieces: &[Piece], at: civil::DateTime) -> String {
    render_placed(pieces, at, Offset::UTC, "UTC")
}

/// [`render`] and [`render_utc`] over the three things a field can read: the
/// civil datetime, the offset, and the zone's name.
fn render_placed(pieces: &[Piece], at: civil::DateTime, offset: Offset, zone: &str) -> String {
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(text) => out.push_str(text),
            Piece::Field(field, count) => {
                render_field(&mut out, *field, *count, at, offset, zone);
            }
        }
    }
    out
}

/// One field of [`render_placed`].
#[expect(
    clippy::cast_sign_loss,
    reason = "every component read here is non-negative by construction but \
              typed `i8`/`i16` by `jiff`; the two that can be negative — the \
              extended year and the modified Julian day — take their absolute \
              value first"
)]
fn render_field(
    out: &mut String,
    field: Field,
    count: usize,
    at: civil::DateTime,
    offset: Offset,
    zone: &str,
) {
    match field {
        Field::Year | Field::ExtendedYear | Field::WeekBasedYear => {
            // One arm for the three, because this subset's `y` is already the
            // signed proleptic year the module doc argues for: what separates
            // them is which year is read — `Y` takes the one its week belongs
            // to — and that `u` has no two-digit window where `y` and `Y` do.
            let year = if field == Field::WeekBasedYear {
                at.date().iso_week_date().year()
            } else {
                at.year()
            };
            if year < 0 {
                out.push('-');
            }
            let magnitude = u32::from(year.unsigned_abs());
            if count == 2 && field != Field::ExtendedYear {
                pad(out, u64::from(magnitude % 100), 2);
            } else {
                pad(out, u64::from(magnitude), count);
            }
        }
        Field::Era => {
            let names = ERAS[usize::from(at.year() <= 0)];
            out.push_str(match count {
                4 => names.0,
                5 => names.2,
                _ => names.1,
            });
        }
        Field::Quarter => {
            let quarter = at.month().unsigned_abs().div_ceil(3);
            match count {
                3 => {
                    out.push('Q');
                    pad(out, u64::from(quarter), 1);
                }
                4 => out.push_str(QUARTERS[usize::from(quarter) - 1]),
                5 => pad(out, u64::from(quarter), 1),
                _ => pad(out, u64::from(quarter), count),
            }
        }
        Field::WeekOfYear => pad(
            out,
            at.date().iso_week_date().week().unsigned_abs().into(),
            count,
        ),
        Field::WeekOfMonth => {
            // The weekday the month opened on, derived rather than looked up:
            // today's weekday walked back over the days already elapsed.
            let day = i64::from(at.day());
            let today = i64::from(at.date().weekday().to_monday_zero_offset());
            let opened = (today - (day - 1)).rem_euclid(7);
            // ISO's minimum of four days, applied to the month: a leading week
            // shorter than that is week 0 rather than week 1.
            let first_is_whole = u64::from(7 - opened >= 4);
            let week = u64::try_from(day + opened - 1).expect("a day of month is one or more") / 7;
            pad(out, week + first_is_whole, count);
        }
        Field::DayOfWeekInMonth => pad(out, u64::from(at.day().unsigned_abs() - 1) / 7 + 1, count),
        Field::StandaloneWeekday | Field::LocalWeekday => {
            let index = at.date().weekday().to_monday_zero_offset() as usize;
            let names = WEEKDAYS[index];
            match count {
                3 => out.push_str(names.1),
                4 => out.push_str(names.0),
                5 => out.push_str(names.2),
                6 => out.push_str(names.3),
                // `c` and `e` both count the day from the one the week starts
                // on, which this module fixes at Monday — the same order
                // [`WEEKDAYS`] is indexed in, so it is the index plus one.
                _ => pad(out, index as u64 + 1, count),
            }
        }
        Field::Month | Field::StandaloneMonth => match count {
            3 => out.push_str(MONTHS[at.month() as usize - 1].1),
            4 => out.push_str(MONTHS[at.month() as usize - 1].0),
            5 => out.push_str(MONTHS[at.month() as usize - 1].2),
            _ => pad(out, at.month() as u64, count),
        },
        Field::Day => pad(out, at.day() as u64, count),
        Field::DayOfYear => pad(out, at.date().day_of_year() as u64, count),
        Field::Weekday => {
            let names = WEEKDAYS[at.date().weekday().to_monday_zero_offset() as usize];
            out.push_str(match count {
                4 => names.0,
                5 => names.2,
                6 => names.3,
                _ => names.1,
            });
        }
        Field::AmPm => out.push_str(if at.hour() < 12 { "AM" } else { "PM" }),
        Field::Hour12 => {
            let hour = at.hour() % 12;
            pad(out, if hour == 0 { 12 } else { hour as u64 }, count);
        }
        Field::Hour23 => pad(out, at.hour() as u64, count),
        Field::Hour11 => pad(out, (at.hour() % 12) as u64, count),
        Field::Hour24 => pad(
            out,
            if at.hour() == 0 { 24 } else { at.hour() as u64 },
            count,
        ),
        Field::Minute => pad(out, at.minute() as u64, count),
        Field::Second => pad(out, at.second() as u64, count),
        Field::Fraction => {
            let mut digits = String::new();
            pad(&mut digits, at.subsec_nanosecond() as u64, 9);
            out.push_str(&digits[..count.min(9)]);
            for _ in 9..count {
                out.push('0');
            }
        }
        Field::OffsetZ | Field::Offset => {
            let seconds = offset.seconds();
            if seconds == 0 && field == Field::OffsetZ {
                out.push('Z');
                return;
            }
            out.push(if seconds < 0 { '-' } else { '+' });
            let total = seconds.unsigned_abs();
            let (hours, minutes) = (u64::from(total / 3_600), u64::from((total % 3_600) / 60));
            pad(out, hours, 2);
            match count {
                1 if minutes == 0 => {}
                3 => {
                    out.push(':');
                    pad(out, minutes, 2);
                }
                _ => pad(out, minutes, 2),
            }
        }
        Field::ZoneId => out.push_str(zone),
        Field::MillisInDay => {
            let millis = at.hour() as u64 * 3_600_000
                + at.minute() as u64 * 60_000
                + at.second() as u64 * 1_000
                + at.subsec_nanosecond() as u64 / 1_000_000;
            pad(out, millis, count);
        }
        Field::JulianDay => {
            let day = julian_day(at.date());
            if day < 0 {
                out.push('-');
            }
            pad(out, day.unsigned_abs().into(), count);
        }
        Field::DayPeriod | Field::FlexibleDayPeriod => {
            let period = day_period(at.time(), field == Field::FlexibleDayPeriod);
            out.push_str(if count == 5 {
                period.narrow
            } else {
                period.name
            });
        }
        // No zone *name* data is transcribed here, so every one of these
        // renders the localized GMT format ICU falls back to when a locale has
        // none — short below `zzzz` and at `v`, long at `zzzz` and `vvvv`.
        Field::ZoneSpecific => localized_gmt(out, offset, count >= 4),
        Field::ZoneGeneric | Field::OffsetGmt => localized_gmt(out, offset, count == 4),
        Field::OffsetBasic => {
            let seconds = offset.seconds();
            if count == 4 {
                localized_gmt(out, offset, true);
            } else if count == 5 && seconds == 0 {
                out.push('Z');
            } else {
                out.push(if seconds < 0 { '-' } else { '+' });
                let total = seconds.unsigned_abs();
                pad(out, u64::from(total / 3_600), 2);
                if count == 5 {
                    out.push(':');
                }
                pad(out, u64::from((total % 3_600) / 60), 2);
            }
        }
    }
}

/// `value`, zero-padded to at least `width` digits.
fn pad(out: &mut String, value: u64, width: usize) {
    let digits = value.to_string();
    for _ in digits.len()..width {
        out.push('0');
    }
    out.push_str(&digits);
}

/// Reads `text` through `pieces` into a civil datetime placed in `zone`.
///
/// The zone comes from the caller rather than from the text, which is what
/// makes a zonal field a refusal here: `Core\Time::parse` takes a `Zone` as
/// its own third argument, so a pattern that also named one would have two
/// answers and no rule to pick between them.
///
/// # Errors
///
/// A one-sentence reason: text that does not match a literal, a field with no
/// digits where it needs them, trailing text nothing consumed, or a
/// combination that is not a real civil time.
pub(crate) fn read(pieces: &[Piece], text: &str, zone: &TimeZone) -> Result<Zoned, String> {
    let mut fields = Fields::default();
    let bytes = text.as_bytes();
    let mut at = 0;
    for piece in pieces {
        match piece {
            Piece::Literal(literal) => {
                if !text[at..].starts_with(literal.as_str()) {
                    return Err(format!("expected `{literal}` at offset {at}"));
                }
                at += literal.len();
            }
            Piece::Field(field, count) => {
                if field.is_zonal() {
                    // Internal consistency: [`civil_fields_only`] refuses this
                    // pattern before any text is read, so an arrival is a
                    // caller of [`read`] that skipped that guard. It is worded
                    // as that guard words it, because the two are one rule.
                    return Err(
                        "a pattern that parses names no zone — `Core\\Time::parse` takes one as \
                         its third argument"
                            .to_owned(),
                    );
                }
                read_field(&mut fields, *field, *count, bytes, &mut at)?;
            }
        }
    }
    if at != bytes.len() {
        return Err(format!("{} characters of trailing text", bytes.len() - at));
    }
    fields.into_zoned(zone)
}

/// What [`read`] accumulates before it has enough to build a value.
#[derive(Default)]
struct Fields {
    year: Option<i16>,
    month: Option<i8>,
    day: Option<i8>,
    hour: Option<i8>,
    minute: Option<i8>,
    second: Option<i8>,
    nanos: Option<i32>,
    /// `true` for PM — applied to [`Self::hour`] only where the pattern read a
    /// 12-hour field, since a `HH` beside an `a` is already unambiguous.
    afternoon: Option<bool>,
    /// Whether the hour came from `h`/`K` rather than `H`/`k`.
    twelve_hour: bool,
}

impl Fields {
    /// The civil time these fields name, placed in `zone`.
    fn into_zoned(self, zone: &TimeZone) -> Result<Zoned, String> {
        let mut hour = self.hour.unwrap_or(0);
        if self.twelve_hour {
            hour %= 12;
            if self.afternoon == Some(true) {
                hour += 12;
            }
        }
        let at = civil::DateTime::new(
            self.year.unwrap_or(1970),
            self.month.unwrap_or(1),
            self.day.unwrap_or(1),
            hour,
            self.minute.unwrap_or(0),
            self.second.unwrap_or(0),
            self.nanos.unwrap_or(0),
        )
        .map_err(|err| err.to_string())?;
        zone.to_zoned(at).map_err(|err| err.to_string())
    }
}

/// One field of [`read`], advancing `at` past what it consumed.
fn read_field(
    fields: &mut Fields,
    field: Field,
    count: usize,
    bytes: &[u8],
    at: &mut usize,
) -> Result<(), String> {
    match field {
        Field::Year => {
            let digits = if count == 2 { 2 } else { count.max(4) };
            let value = number(bytes, at, if count == 2 { 2 } else { 1 }, digits, "year")?;
            // A two-digit year lands in the 1969–2068 window, which is POSIX's
            // rule and the one PHP's own `y` uses; every other count is the
            // year itself.
            let year = if count == 2 {
                if value <= 68 {
                    value + 2_000
                } else {
                    value + 1_900
                }
            } else {
                value
            };
            fields.year = Some(i16::try_from(year).map_err(|_| "year out of range".to_owned())?);
        }
        Field::ExtendedYear => {
            // The one field that reads a sign, which is the whole of what `u`
            // is for here: `y` reads digits, so a date before year 1 comes back
            // through this letter and no other.
            let negative = bytes.get(*at) == Some(&b'-');
            if negative {
                *at += 1;
            }
            let value = number(bytes, at, 1, count.max(4), "year")?;
            let year = if negative { -value } else { value };
            fields.year = Some(i16::try_from(year).map_err(|_| "year out of range".to_owned())?);
        }
        Field::Era => {
            // Read and discarded, for [`Field::DayOfYear`]'s reason: the era of
            // a signed proleptic year is a function of its sign, so the text
            // can only agree with it or contradict it.
            name_index(bytes, at, "era", ERAS.len(), |index| {
                let names = ERAS[index];
                [names.0, names.1, names.2]
            })?;
        }
        Field::Quarter if count == 4 => {
            // One name per quarter rather than three, so the roster is written
            // as the shape [`name_index`] takes with its alternatives spent.
            name_index(bytes, at, "quarter", QUARTERS.len(), |index| {
                [QUARTERS[index], QUARTERS[index], QUARTERS[index]]
            })?;
        }
        Field::Quarter if count == 3 => {
            if bytes
                .get(*at)
                .is_none_or(|byte| !byte.eq_ignore_ascii_case(&b'Q'))
            {
                return Err(format!("expected a quarter at offset {at}"));
            }
            *at += 1;
            number(bytes, at, 1, 1, "quarter")?;
        }
        // Read and discarded, for [`Field::DayOfYear`]'s reason: each is a
        // function of the date the other fields name, so the text can only
        // agree with it or contradict it.
        Field::Quarter | Field::WeekOfYear | Field::WeekOfMonth | Field::DayOfWeekInMonth => {
            number(bytes, at, 1, count.max(2), "week")?;
        }
        Field::StandaloneWeekday | Field::LocalWeekday if count >= 3 => {
            name_index(bytes, at, "weekday", WEEKDAYS.len(), |index| {
                let names = WEEKDAYS[index];
                [names.0, names.1, names.3]
            })?;
        }
        Field::StandaloneWeekday | Field::LocalWeekday => {
            small(bytes, at, count, "weekday")?;
        }
        Field::WeekBasedYear => {
            // Read and discarded, for [`Field::DayOfYear`]'s reason: the year
            // a week belongs to is a function of the date, and a date is built
            // here from `y`, `M` and `d` rather than from a week and a day in
            // it.
            let digits = if count == 2 { 2 } else { count.max(4) };
            number(bytes, at, 1, digits, "week-based year")?;
        }
        Field::Month | Field::StandaloneMonth if count >= 3 => {
            let index = name_index(bytes, at, "month", MONTHS.len(), |index| {
                let names = MONTHS[index];
                [names.0, names.1, names.2]
            })? + 1;
            fields.month =
                Some(i8::try_from(index).expect("a month index is between one and twelve"));
        }
        Field::Month | Field::StandaloneMonth => {
            fields.month = Some(small(bytes, at, count, "month")?);
        }
        Field::Day => fields.day = Some(small(bytes, at, count, "day")?),
        Field::DayOfYear => {
            // Read and discarded: a day-of-year beside a month and a day would
            // be a second answer for the same thing, and alone it is not a
            // civil date this subset builds from.
            number(bytes, at, 1, count.max(3), "day of year")?;
        }
        Field::Weekday => {
            // Read and discarded for the reason above: the weekday of a civil
            // date is a function of the date, so the text can only agree or
            // contradict, and PHP's own parser ignores it too.
            name_index(bytes, at, "weekday", WEEKDAYS.len(), |index| {
                let names = WEEKDAYS[index];
                [names.0, names.1, names.3]
            })?;
        }
        Field::AmPm => {
            let rest = &bytes[*at..];
            let marker = rest
                .get(..2)
                .ok_or_else(|| "expected AM or PM".to_owned())?;
            fields.afternoon = Some(match marker.to_ascii_uppercase().as_slice() {
                b"AM" => false,
                b"PM" => true,
                _ => return Err("expected AM or PM".to_owned()),
            });
            *at += 2;
        }
        Field::Hour12 | Field::Hour11 => {
            fields.twelve_hour = true;
            fields.hour = Some(small(bytes, at, count, "hour")?);
        }
        Field::Hour23 | Field::Hour24 => {
            let hour = small(bytes, at, count, "hour")?;
            // `k` counts 1–24, so its midnight is written `24` and is the same
            // civil hour `H` writes `0`.
            fields.hour = Some(if field == Field::Hour24 && hour == 24 {
                0
            } else {
                hour
            });
        }
        Field::Minute => fields.minute = Some(small(bytes, at, count, "minute")?),
        Field::Second => fields.second = Some(small(bytes, at, count, "second")?),
        Field::Fraction => {
            let read = number(bytes, at, 1, count, "fractional second")?;
            let digits = u32::try_from(count.min(9)).expect("a count below ten fits a `u32`");
            let scale = 10_i64.pow(9 - digits);
            fields.nanos =
                Some(i32::try_from(read * scale).map_err(|_| "fraction out of range".to_owned())?);
        }
        Field::MillisInDay => {
            // Eight digits is the whole of a day, so a wider count reads no
            // further than one.
            let millis = number(bytes, at, 1, count.max(8), "milliseconds in the day")?;
            fields.hour = Some(
                i8::try_from(millis / 3_600_000)
                    .map_err(|_| "milliseconds in the day out of range".to_owned())?,
            );
            fields.minute =
                Some(i8::try_from(millis / 60_000 % 60).expect("a minute of an hour fits an `i8`"));
            fields.second =
                Some(i8::try_from(millis / 1_000 % 60).expect("a second of a minute fits an `i8`"));
            fields.nanos = Some(
                i32::try_from(millis % 1_000 * 1_000_000)
                    .expect("a millisecond in nanoseconds fits an `i32`"),
            );
        }
        Field::JulianDay => {
            // Signed, for [`Field::ExtendedYear`]'s reason: the count is
            // negative for every date before 1858, which is inside the range a
            // `Core\Time\Date` accepts.
            let negative = bytes.get(*at) == Some(&b'-');
            if negative {
                *at += 1;
            }
            let read = number(bytes, at, 1, count.max(7), "modified Julian day")?;
            let out_of_range = || "modified Julian day out of range".to_owned();
            let date = jiff::Span::new()
                .try_days(if negative { -read } else { read })
                .and_then(|days| JULIAN_EPOCH.checked_add(days))
                .map_err(|_| out_of_range())?;
            fields.year = Some(date.year());
            fields.month = Some(date.month());
            fields.day = Some(date.day());
        }
        Field::DayPeriod | Field::FlexibleDayPeriod => {
            let roster: &[DayPeriod] = if field == Field::DayPeriod {
                &DAY_PERIODS
            } else {
                &FLEXIBLE_DAY_PERIODS
            };
            let index = name_index(bytes, at, "day period", roster.len(), |index| {
                let period = roster[index];
                [period.name, period.narrow, period.narrow]
            })?;
            // A period that names an instant fixes the hour outright, and one
            // that names a range answers the question `a` answers — which is
            // all a 12-hour field beside it needs.
            let period = roster[index];
            fields.afternoon = Some(period.afternoon);
            if let Some(hour) = period.hour {
                fields.hour = Some(hour);
            }
        }
        Field::OffsetZ
        | Field::Offset
        | Field::ZoneId
        | Field::ZoneSpecific
        | Field::ZoneGeneric
        | Field::OffsetBasic
        | Field::OffsetGmt => {
            unreachable!("a zonal field is refused before it reaches here")
        }
    }
    Ok(())
}

/// A component small enough for an `i8` — every one but the year.
fn small(bytes: &[u8], at: &mut usize, count: usize, what: &str) -> Result<i8, String> {
    let value = number(
        bytes,
        at,
        if count > 1 { count } else { 1 },
        count.max(2),
        what,
    )?;
    i8::try_from(value).map_err(|_| format!("{what} out of range"))
}

/// Reads between `least` and `most` ASCII digits, advancing `at`.
fn number(
    bytes: &[u8],
    at: &mut usize,
    least: usize,
    most: usize,
    what: &str,
) -> Result<i64, String> {
    let mut value: i64 = 0;
    let mut read = 0;
    while read < most {
        let Some(byte) = bytes.get(*at).copied().filter(u8::is_ascii_digit) else {
            break;
        };
        value = value * 10 + i64::from(byte - b'0');
        *at += 1;
        read += 1;
    }
    if read < least {
        return Err(format!(
            "expected {least} or more digits of {what} at offset {at}"
        ));
    }
    Ok(value)
}

/// Matches one of the names `names` gives for each of `roster` indices, longest
/// first so that `"June"` is not read as the short `"Jun"` with a stray `e`
/// after it.
///
/// The answer is the **zero-based** index; a caller wanting a one-based
/// component adds the one itself, which is the month and nothing else.
fn name_index(
    bytes: &[u8],
    at: &mut usize,
    what: &str,
    roster: usize,
    names: impl Fn(usize) -> [&'static str; 3],
) -> Result<usize, String> {
    let mut best: Option<(usize, usize)> = None;
    for index in 0..roster {
        for name in names(index) {
            let matched = bytes
                .get(*at..*at + name.len())
                .is_some_and(|found| found.eq_ignore_ascii_case(name.as_bytes()));
            if matched && best.is_none_or(|(_, length)| name.len() > length) {
                best = Some((index, name.len()));
            }
        }
    }
    let (index, length) = best.ok_or_else(|| format!("expected a {what} name at offset {at}"))?;
    *at += length;
    Ok(index)
}

// ============================================================================
// The plural rules — `rule:programs/framework-core-half`'s `Core\Cldr` row
// ============================================================================

/// `Core\Cldr`'s name, spelled once.
pub(crate) const NAME: &str = r"Core\Cldr";

/// [`PLURAL_CATEGORY`]'s name, spelled once.
pub(crate) const PLURAL_CATEGORY_NAME: &str = r"Core\Cldr\PluralCategory";

/// The cardinal member's name, as its own refusals spell it.
const PLURAL_MEMBER: &str = r"Core\Cldr::pluralCategory";

/// The ordinal member's name, likewise.
const ORDINAL_MEMBER: &str = r"Core\Cldr::ordinalCategory";

/// `rule:programs/framework-core-half`'s row and the ordinal table beside it, and the whole of the
/// class: two members, no capability, no instance and no constant.
///
/// It declares no capability for the reason [`crate::storage`] declares none
/// and a stronger one: nothing here reaches outside the process at all. The
/// answer is a function of two arguments and a table compiled into the binary,
/// so `rule:security/capability-question-is-grant-and-scope`
/// has no door to put a check at.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "pluralCategory",
            names: &["count", "locale"],
            params: &[
                CoreTy::Union(crate::math::NUMBER),
                CoreTy::Text(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Enum(PLURAL_CATEGORY_NAME),
            symbol: "nvs_core_cldr_plural_category",
            doc: Some(&PLURAL_CATEGORY_MEMBER_DOC),
        },
        CoreMethod {
            name: "ordinalCategory",
            names: &["count", "locale"],
            params: &[
                CoreTy::Union(crate::math::NUMBER),
                CoreTy::Text(Qual::Neutral),
            ],
            defaults: &[],
            return_ty: CoreTy::Enum(PLURAL_CATEGORY_NAME),
            symbol: "nvs_core_cldr_ordinal_category",
            doc: Some(&ORDINAL_CATEGORY_MEMBER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Cldr::pluralCategory`'s reference card — `rule:core-api/reference-card`.
const PLURAL_CATEGORY_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Answers which of CLDR's plural forms `$count` selects in `$locale`, so a message \
            catalog keys its variants on the locale's own rule rather than on `== 1`.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "The number the message is about. A `decimal` carries its scale, so `1.0` and \
                   `1` can select different forms where a locale reads the fraction; an `int` has \
                   none, and a `float` is read at the digits it prints.",
            shape: &[],
        },
        ParamDoc {
            name: "locale",
            desc: "A BCP 47 tag. Only the language subtag is read and case is ignored, so `en-GB`, \
                   `en_US` and `EN` all answer as `en`.",
            shape: &[],
        },
    ],
    ret: "The category the language's rules put `$count` in — `Other` for every count in a \
          language that makes no plural distinction.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The tag carries no language subtag, or names a language whose rules are not \
                   among those compiled in — a locale is never given another language's rules.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is a `float` that is not finite, or one that prints more digits than \
                   the rules can be evaluated over.",
        },
    ],
};

/// `Core\Cldr::ordinalCategory`'s reference card — `rule:core-api/reference-card`.
const ORDINAL_CATEGORY_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Answers which form `$count` takes as a *place* rather than as an amount — English's \
            `1st`, `2nd`, `3rd`, `4th` — so a template writes the suffix its locale marks.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "The place the message is about. CLDR states its ordinal rules over whole \
                   numbers, so a count showing a fraction is outside all of them and answers \
                   `Other` — except in the two languages whose rules read the integer part \
                   alone, Macedonian and Georgian.",
            shape: &[],
        },
        ParamDoc {
            name: "locale",
            desc: "A BCP 47 tag, read exactly as `pluralCategory` reads it: only the language \
                   subtag, and case is ignored.",
            shape: &[],
        },
    ],
    ret: "The category the language's ordinal rules put `$count` in — `Other` for every count in \
          a language that marks no ordinal form, which is most of them.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "The tag carries no language subtag, or names a language neither table carries \
                   rules for. A language carried for cardinals but marking no ordinal form is not \
                   this case: it answers `Other`.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$count` is a `float` that is not finite, or one that prints more digits than \
                   the rules can be evaluated over.",
        },
    ],
};

/// CLDR's six plural categories, in CLDR's own order.
///
/// Registered rather than left to a `string` because the roster is closed and
/// the names mean nothing on their own: `One` is whatever form a language uses
/// for the counts its rules put there, which in Russian includes 21 and
/// excludes 11.
pub(crate) const PLURAL_CATEGORY: CoreEnum = CoreEnum {
    name: PLURAL_CATEGORY_NAME,
    cases: &[
        ("Zero", 0),
        ("One", 1),
        ("Two", 2),
        ("Few", 3),
        ("Many", 4),
        ("Other", 5),
    ],
    doc: Some(&PLURAL_CATEGORY_DOC),
};

/// [`PLURAL_CATEGORY`]'s reference card — `rule:core-api/reference-card`.
const PLURAL_CATEGORY_DOC: EnumDoc = EnumDoc {
    short: "Which of CLDR's six plural forms a count selects. The names are CLDR's own labels for \
            a language's forms, not counts: only `Other` means the same thing everywhere, and a \
            language uses as few of the six as its grammar needs.",
    cases: &[
        CaseDoc {
            name: "Zero",
            desc: "The form a language keeps for none of something — Arabic and Welsh have one; \
                   most languages do not.",
        },
        CaseDoc {
            name: "One",
            desc: "The singular, as that language draws it: English's 1, Russian's 1, 21 and 31, \
                   French's 0 and 1.",
        },
        CaseDoc {
            name: "Two",
            desc: "The dual — Arabic, Welsh, Slovenian and Irish among the languages carried.",
        },
        CaseDoc {
            name: "Few",
            desc: "The paucal, for the small counts a language groups: Russian's 2 to 4, Arabic's \
                   3 to 10, Welsh's 3 alone.",
        },
        CaseDoc {
            name: "Many",
            desc: "The form above `Few` where a language has both — Russian's 5 to 20, and the \
                   whole millions in Romance languages that mark them.",
        },
        CaseDoc {
            name: "Other",
            desc: "The form every language has, and in a language with no plural distinction the \
                   only one any count selects.",
        },
    ],
};

/// One of CLDR's six plural categories — [`PLURAL_CATEGORY`]'s cases, as the
/// rules below answer with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Category {
    /// The constant an enum value *is* at run time (`rule:enums/closed-integer-type`), which is what
    /// the member hands back.
    ///
    /// Written here and checked against [`PLURAL_CATEGORY`] by
    /// `an_ordinal_is_its_case_in_the_registered_enum` rather than looked up
    /// per call: the roster is six wide and closed, so a scan would buy
    /// nothing a test does not already buy once.
    fn ordinal(self) -> i64 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Two => 2,
            Self::Few => 3,
            Self::Many => 4,
            Self::Other => 5,
        }
    }
}

/// CLDR's plural operands for one count, holding the three the carried rules
/// read.
///
/// `n` is derived rather than stored ([`Operands::n`]), and `t` — `f` without
/// its trailing zeros — is never needed as a number: every rule here asks only
/// whether it is zero, which is `f == 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Operands {
    /// `i` — the integer part of the absolute value.
    i: u128,
    /// `v` — how many fraction digits the count *shows*, trailing zeros
    /// included.
    v: u32,
    /// `f` — those digits read as an integer, trailing zeros included.
    f: u128,
}

impl Operands {
    /// `n` — the absolute value — but only when it is a whole number.
    ///
    /// CLDR matches `n` against integers and integer ranges, so a count with a
    /// non-zero fraction is outside every one of them rather than rounded into
    /// one: `n = 1` is false for `1.5` and `n = 3..10` is false for `3.5`.
    /// Answering `None` for those is what lets each predicate below read like
    /// its published rule.
    fn n(self) -> Option<u128> {
        (self.f == 0).then_some(self.i)
    }

    /// CLDR's `n = value`.
    fn n_is(self, value: u128) -> bool {
        self.n() == Some(value)
    }

    /// CLDR's `n = low..high`.
    fn n_in(self, range: RangeInclusive<u128>) -> bool {
        self.n().is_some_and(|n| range.contains(&n))
    }

    /// CLDR's `n = a,b,c` — the ordinal rules' commonest shape, where the
    /// cardinal ones mostly write ranges and moduli.
    fn n_any(self, values: &[u128]) -> bool {
        self.n().is_some_and(|n| values.contains(&n))
    }

    /// CLDR's `n % modulus = value`.
    fn n_mod_is(self, modulus: u128, value: u128) -> bool {
        self.n().is_some_and(|n| n % modulus == value)
    }

    /// CLDR's `n % modulus = low..high`.
    fn n_mod_in(self, modulus: u128, range: RangeInclusive<u128>) -> bool {
        self.n().is_some_and(|n| range.contains(&(n % modulus)))
    }

    /// CLDR's `n % modulus = a,b,c` — the published alternative to a range,
    /// which Breton and Cornish write and no other carried rule does.
    fn n_mod_any(self, modulus: u128, values: &[u128]) -> bool {
        self.n().is_some_and(|n| values.contains(&(n % modulus)))
    }

    /// The `many` arm `ca`, `es`, `fr`, `it` and `pt` share: a whole number of
    /// millions, and not zero.
    ///
    /// The published rule reads `e = 0 and i != 0 and i % 1000000 = 0 and
    /// v = 0 or e != 0..5`; `e` is 0 here always (the module doc says why), so
    /// what is left is the first half.
    fn is_whole_millions(self) -> bool {
        self.i != 0 && self.i.is_multiple_of(1_000_000) && self.v == 0
    }
}

/// One CLDR cardinal rule set, named for a language that uses it or for the
/// shape it has.
///
/// A rule set rather than a locale is the unit because CLDR's own data is
/// shaped that way: the languages in [`RULES`] share a few dozen sets between
/// them, so every language on a set shares one implementation of it and a new
/// language is usually a row rather than an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleSet {
    /// No plural distinction at all: every count is `Other`.
    NoDistinction,
    /// `one: i = 1 and v = 0` — English, and the set with the most languages.
    Unit,
    /// [`Self::Unit`] plus the millions `many`.
    UnitAndMillions,
    /// `one: n = 1` — [`Self::Unit`]'s neighbour, and the difference is `1.0`.
    ExactlyOne,
    /// [`Self::ExactlyOne`] plus the millions `many` — Spanish.
    ExactlyOneAndMillions,
    /// `one: i = 0 or n = 1` — Hindi, Bengali, Persian, Zulu.
    ZeroOrExactlyOne,
    /// `one: i = 0,1` — Armenian, Fulah, Kabyle. Unlike
    /// [`Self::ZeroOrExactlyOne`] this puts `1.5` in `One`, since it reads `i`
    /// and not `n`.
    IntegerZeroOrOne,
    /// [`Self::IntegerZeroOrOne`] plus the millions `many` — French,
    /// Portuguese.
    IntegerZeroOrOneAndMillions,
    /// Russian and Ukrainian: `one`/`few`/`many` off `i`, and every count with
    /// a fraction is `Other`.
    EastSlavic,
    /// Polish, whose `many` differs from [`Self::EastSlavic`]'s at 1 and at
    /// 12–14.
    Polish,
    /// Czech and Slovak, whose `many` is the fraction itself.
    Czech,
    /// Bosnian, Croatian, Serbian: [`Self::EastSlavic`]'s shape with no
    /// `many`, and with the same test applied to the fraction digits.
    SerboCroatian,
    /// Arabic, the language that uses all six.
    Arabic,
    /// Welsh, which uses all six off bare counts — the clearest statement that
    /// these names are labels and not amounts.
    Welsh,
    /// Lithuanian, whose `many` is the fraction.
    Lithuanian,
    /// Latvian, the one carried set with a `zero` that is not `n = 0`.
    Latvian,
    /// Romanian, whose `few` swallows the fraction, zero and 2–19 alike.
    Romanian,
    /// Slovenian, which reads `i % 100` where the Slavic sets read `i % 10`.
    Slovenian,
    /// Irish, whose categories are four contiguous bands.
    Irish,
    /// Belarusian: [`Self::EastSlavic`]'s three bands read off `n` rather than
    /// off `i`, so a count showing a fraction falls out of all of them into
    /// `Other` instead of being read by its integer part.
    Belarusian,
    /// Hebrew, whose `one` also takes a count with no integer part at all
    /// (`0.5`), and which distinguishes two and nothing beyond it.
    Hebrew,
    /// Maltese, whose `few` starts at zero and whose `many` is the teens.
    Maltese,
    /// Lower and Upper Sorbian: [`Self::Slovenian`]'s three bands with the
    /// same test applied to the fraction digits, as [`Self::SerboCroatian`]
    /// does to [`Self::EastSlavic`]'s.
    Sorbian,
    /// Scottish Gaelic, whose first three bands each repeat once in the teens.
    ScottishGaelic,
    /// Breton, which excludes three whole tens-bands from each of its first
    /// three categories and keeps a millions `many` behind them.
    Breton,
    /// Cornish, the widest published rule carried here: five categories, four
    /// moduli and a range inside one of them.
    Cornish,
    /// Manx, whose `many` is any visible fraction and whose `few` is the even
    /// twenties.
    Manx,
    /// Icelandic: [`Self::EastSlavic`]'s `one` test on `i`, with every count
    /// showing a non-zero fraction joining `one` rather than leaving it.
    Icelandic,
    /// Macedonian: [`Self::SerboCroatian`]'s `one` with no `few` behind it.
    Macedonian,
    /// Central Atlas Tamazight, whose `one` is two disjoint bands.
    Tamazight,
    /// Tachelhit, which is [`Self::ZeroOrExactlyOne`]'s `one` with a `few`
    /// behind it.
    Tachelhit,
    /// Sinhala, whose `one` takes `0.1` and no other fraction.
    Sinhala,
    /// `one: n = 0..1` — Akan, Bihari, Gun, Northern Sotho, Walloon. Unlike
    /// [`Self::IntegerZeroOrOne`] this reads `n`, so `1.5` is `Other` here and
    /// `One` there.
    ZeroToOne,
    /// Nama, which distinguishes one and two and nothing else.
    OneAndTwo,
    /// Danish, whose `one` takes any count below two that shows a fraction —
    /// `0.5` and `1.5` alike, which no other carried set does.
    Danish,
    /// Filipino, Tagalog and Cebuano, whose `one` is stated as three
    /// alternatives over the last digit rather than as a band.
    Filipino,
}

impl RuleSet {
    /// CLDR's own rule for this set, one arm each, in the order the categories
    /// are published in: the first match wins, and `Other` is the
    /// fallthrough every set has.
    #[allow(clippy::match_same_arms)]
    fn select(self, at: Operands) -> Category {
        use Category::{Few, Many, One, Other, Two, Zero};
        match self {
            Self::NoDistinction => Other,

            Self::Unit if at.i == 1 && at.v == 0 => One,
            Self::Unit => Other,

            Self::UnitAndMillions if at.i == 1 && at.v == 0 => One,
            Self::UnitAndMillions if at.is_whole_millions() => Many,
            Self::UnitAndMillions => Other,

            Self::ExactlyOne if at.n_is(1) => One,
            Self::ExactlyOne => Other,

            Self::ExactlyOneAndMillions if at.n_is(1) => One,
            Self::ExactlyOneAndMillions if at.is_whole_millions() => Many,
            Self::ExactlyOneAndMillions => Other,

            Self::ZeroOrExactlyOne if at.i == 0 || at.n_is(1) => One,
            Self::ZeroOrExactlyOne => Other,

            Self::IntegerZeroOrOne if at.i <= 1 => One,
            Self::IntegerZeroOrOne => Other,

            Self::IntegerZeroOrOneAndMillions if at.i <= 1 => One,
            Self::IntegerZeroOrOneAndMillions if at.is_whole_millions() => Many,
            Self::IntegerZeroOrOneAndMillions => Other,

            // one:  v = 0 and i % 10 = 1 and i % 100 != 11
            // few:  v = 0 and i % 10 = 2..4 and i % 100 != 12..14
            // many: v = 0 and i % 10 = 0 or v = 0 and i % 10 = 5..9
            //       or v = 0 and i % 100 = 11..14
            Self::EastSlavic if at.v != 0 => Other,
            Self::EastSlavic if at.i % 10 == 1 && at.i % 100 != 11 => One,
            Self::EastSlavic
                if (2..=4).contains(&(at.i % 10)) && !(12..=14).contains(&(at.i % 100)) =>
            {
                Few
            }
            Self::EastSlavic
                if at.i.is_multiple_of(10)
                    || (5..=9).contains(&(at.i % 10))
                    || (11..=14).contains(&(at.i % 100)) =>
            {
                Many
            }
            Self::EastSlavic => Other,

            // one:  i = 1 and v = 0
            // few:  v = 0 and i % 10 = 2..4 and i % 100 != 12..14
            // many: v = 0 and i != 1 and i % 10 = 0..1 or v = 0 and i % 10 = 5..9
            //       or v = 0 and i % 100 = 12..14
            Self::Polish if at.i == 1 && at.v == 0 => One,
            Self::Polish if at.v != 0 => Other,
            Self::Polish
                if (2..=4).contains(&(at.i % 10)) && !(12..=14).contains(&(at.i % 100)) =>
            {
                Few
            }
            Self::Polish
                if (at.i != 1 && (0..=1).contains(&(at.i % 10)))
                    || (5..=9).contains(&(at.i % 10))
                    || (12..=14).contains(&(at.i % 100)) =>
            {
                Many
            }
            Self::Polish => Other,

            // one: i = 1 and v = 0 / few: i = 2..4 and v = 0 / many: v != 0
            Self::Czech if at.v == 0 && at.i == 1 => One,
            Self::Czech if at.v == 0 && (2..=4).contains(&at.i) => Few,
            Self::Czech if at.v != 0 => Many,
            Self::Czech => Other,

            // one: v = 0 and i % 10 = 1 and i % 100 != 11 or f % 10 = 1 and f % 100 != 11
            // few: v = 0 and i % 10 = 2..4 and i % 100 != 12..14
            //      or f % 10 = 2..4 and f % 100 != 12..14
            Self::SerboCroatian
                if (at.v == 0 && at.i % 10 == 1 && at.i % 100 != 11)
                    || (at.f % 10 == 1 && at.f % 100 != 11) =>
            {
                One
            }
            Self::SerboCroatian
                if (at.v == 0
                    && (2..=4).contains(&(at.i % 10))
                    && !(12..=14).contains(&(at.i % 100)))
                    || ((2..=4).contains(&(at.f % 10)) && !(12..=14).contains(&(at.f % 100))) =>
            {
                Few
            }
            Self::SerboCroatian => Other,

            // zero: n = 0 / one: n = 1 / two: n = 2
            // few: n % 100 = 3..10 / many: n % 100 = 11..99
            Self::Arabic if at.n_is(0) => Zero,
            Self::Arabic if at.n_is(1) => One,
            Self::Arabic if at.n_is(2) => Two,
            Self::Arabic if at.n_mod_in(100, 3..=10) => Few,
            Self::Arabic if at.n_mod_in(100, 11..=99) => Many,
            Self::Arabic => Other,

            // zero: n = 0 / one: n = 1 / two: n = 2 / few: n = 3 / many: n = 6
            Self::Welsh if at.n_is(0) => Zero,
            Self::Welsh if at.n_is(1) => One,
            Self::Welsh if at.n_is(2) => Two,
            Self::Welsh if at.n_is(3) => Few,
            Self::Welsh if at.n_is(6) => Many,
            Self::Welsh => Other,

            // one:  n % 10 = 1 and n % 100 != 11..19
            // few:  n % 10 = 2..9 and n % 100 != 11..19
            // many: f != 0
            Self::Lithuanian if at.n_mod_is(10, 1) && !at.n_mod_in(100, 11..=19) => One,
            Self::Lithuanian if at.n_mod_in(10, 2..=9) && !at.n_mod_in(100, 11..=19) => Few,
            Self::Lithuanian if at.f != 0 => Many,
            Self::Lithuanian => Other,

            // zero: n % 10 = 0 or n % 100 = 11..19 or v = 2 and f % 100 = 11..19
            // one:  n % 10 = 1 and n % 100 != 11 or v = 2 and f % 10 = 1 and f % 100 != 11
            //       or v != 2 and f % 10 = 1
            Self::Latvian
                if at.n_mod_is(10, 0)
                    || at.n_mod_in(100, 11..=19)
                    || (at.v == 2 && (11..=19).contains(&(at.f % 100))) =>
            {
                Zero
            }
            Self::Latvian
                if (at.n_mod_is(10, 1) && !at.n_mod_is(100, 11))
                    || (at.v == 2 && at.f % 10 == 1 && at.f % 100 != 11)
                    || (at.v != 2 && at.f % 10 == 1) =>
            {
                One
            }
            Self::Latvian => Other,

            // one: i = 1 and v = 0 / few: v != 0 or n = 0 or n % 100 = 2..19
            Self::Romanian if at.i == 1 && at.v == 0 => One,
            Self::Romanian if at.v != 0 || at.n_is(0) || at.n_mod_in(100, 2..=19) => Few,
            Self::Romanian => Other,

            // one: v = 0 and i % 100 = 1 / two: v = 0 and i % 100 = 2
            // few: v = 0 and i % 100 = 3..4 or v != 0
            Self::Slovenian if at.v == 0 && at.i % 100 == 1 => One,
            Self::Slovenian if at.v == 0 && at.i % 100 == 2 => Two,
            Self::Slovenian if (at.v == 0 && (3..=4).contains(&(at.i % 100))) || at.v != 0 => Few,
            Self::Slovenian => Other,

            // one: n = 1 / two: n = 2 / few: n = 3..6 / many: n = 7..10
            Self::Irish if at.n_is(1) => One,
            Self::Irish if at.n_is(2) => Two,
            Self::Irish if at.n_in(3..=6) => Few,
            Self::Irish if at.n_in(7..=10) => Many,
            Self::Irish => Other,

            // one:  n % 10 = 1 and n % 100 != 11
            // few:  n % 10 = 2..4 and n % 100 != 12..14
            // many: n % 10 = 0 or n % 10 = 5..9 or n % 100 = 11..14
            Self::Belarusian if at.n_mod_is(10, 1) && !at.n_mod_is(100, 11) => One,
            Self::Belarusian if at.n_mod_in(10, 2..=4) && !at.n_mod_in(100, 12..=14) => Few,
            Self::Belarusian
                if at.n_mod_is(10, 0) || at.n_mod_in(10, 5..=9) || at.n_mod_in(100, 11..=14) =>
            {
                Many
            }
            Self::Belarusian => Other,

            // one: i = 1 and v = 0 or i = 0 and v != 0 / two: i = 2 and v = 0
            Self::Hebrew if (at.i == 1 && at.v == 0) || (at.i == 0 && at.v != 0) => One,
            Self::Hebrew if at.i == 2 && at.v == 0 => Two,
            Self::Hebrew => Other,

            // one: n = 1 / two: n = 2 / few: n = 0 or n % 100 = 3..10
            // many: n % 100 = 11..19
            Self::Maltese if at.n_is(1) => One,
            Self::Maltese if at.n_is(2) => Two,
            Self::Maltese if at.n_is(0) || at.n_mod_in(100, 3..=10) => Few,
            Self::Maltese if at.n_mod_in(100, 11..=19) => Many,
            Self::Maltese => Other,

            // one: v = 0 and i % 100 = 1 or f % 100 = 1
            // two: v = 0 and i % 100 = 2 or f % 100 = 2
            // few: v = 0 and i % 100 = 3..4 or f % 100 = 3..4
            Self::Sorbian if (at.v == 0 && at.i % 100 == 1) || at.f % 100 == 1 => One,
            Self::Sorbian if (at.v == 0 && at.i % 100 == 2) || at.f % 100 == 2 => Two,
            Self::Sorbian
                if (at.v == 0 && (3..=4).contains(&(at.i % 100)))
                    || (3..=4).contains(&(at.f % 100)) =>
            {
                Few
            }
            Self::Sorbian => Other,

            // one: n = 1,11 / two: n = 2,12 / few: n = 3..10,13..19
            Self::ScottishGaelic if at.n_is(1) || at.n_is(11) => One,
            Self::ScottishGaelic if at.n_is(2) || at.n_is(12) => Two,
            Self::ScottishGaelic if at.n_in(3..=10) || at.n_in(13..=19) => Few,
            Self::ScottishGaelic => Other,

            // one:  n % 10 = 1 and n % 100 != 11,71,91
            // two:  n % 10 = 2 and n % 100 != 12,72,92
            // few:  n % 10 = 3..4,9 and n % 100 != 10..19,70..79,90..99
            // many: n != 0 and n % 1000000 = 0
            Self::Breton if at.n_mod_is(10, 1) && !at.n_mod_any(100, &[11, 71, 91]) => One,
            Self::Breton if at.n_mod_is(10, 2) && !at.n_mod_any(100, &[12, 72, 92]) => Two,
            Self::Breton
                if (at.n_mod_in(10, 3..=4) || at.n_mod_is(10, 9))
                    && !(at.n_mod_in(100, 10..=19)
                        || at.n_mod_in(100, 70..=79)
                        || at.n_mod_in(100, 90..=99)) =>
            {
                Few
            }
            Self::Breton if !at.n_is(0) && at.n_mod_is(1_000_000, 0) => Many,
            Self::Breton => Other,

            // zero: n = 0 / one: n = 1
            // two:  n % 100 = 2,22,42,62,82
            //       or n % 1000 = 0 and n % 100000 = 1000..20000,40000,60000,80000
            //       or n != 0 and n % 1000000 = 100000
            // few:  n % 100 = 3,23,43,63,83
            // many: n != 1 and n % 100 = 1,21,41,61,81
            Self::Cornish if at.n_is(0) => Zero,
            Self::Cornish if at.n_is(1) => One,
            Self::Cornish
                if at.n_mod_any(100, &[2, 22, 42, 62, 82])
                    || (at.n_mod_is(1_000, 0)
                        && (at.n_mod_in(100_000, 1_000..=20_000)
                            || at.n_mod_any(100_000, &[40_000, 60_000, 80_000])))
                    || (!at.n_is(0) && at.n_mod_is(1_000_000, 100_000)) =>
            {
                Two
            }
            Self::Cornish if at.n_mod_any(100, &[3, 23, 43, 63, 83]) => Few,
            Self::Cornish if !at.n_is(1) && at.n_mod_any(100, &[1, 21, 41, 61, 81]) => Many,
            Self::Cornish => Other,

            // one: v = 0 and i % 10 = 1 / two: v = 0 and i % 10 = 2
            // few: v = 0 and i % 100 = 0,20,40,60,80 / many: v != 0
            Self::Manx if at.v == 0 && at.i % 10 == 1 => One,
            Self::Manx if at.v == 0 && at.i % 10 == 2 => Two,
            Self::Manx if at.v == 0 && matches!(at.i % 100, 0 | 20 | 40 | 60 | 80) => Few,
            Self::Manx if at.v != 0 => Many,
            Self::Manx => Other,

            // one: t = 0 and i % 10 = 1 and i % 100 != 11 or t != 0
            // `t` is `f` with its trailing zeros dropped, so `t != 0` is
            // `f != 0` — the equivalence [`Operands`]'s doc states.
            Self::Icelandic if at.f != 0 || (at.i % 10 == 1 && at.i % 100 != 11) => One,
            Self::Icelandic => Other,

            // one: v = 0 and i % 10 = 1 and i % 100 != 11
            //      or f % 10 = 1 and f % 100 != 11
            Self::Macedonian
                if (at.v == 0 && at.i % 10 == 1 && at.i % 100 != 11)
                    || (at.f % 10 == 1 && at.f % 100 != 11) =>
            {
                One
            }
            Self::Macedonian => Other,

            // one: n = 0..1 or n = 11..99
            Self::Tamazight if at.n_in(0..=1) || at.n_in(11..=99) => One,
            Self::Tamazight => Other,

            // one: i = 0 or n = 1 / few: n = 2..10
            Self::Tachelhit if at.i == 0 || at.n_is(1) => One,
            Self::Tachelhit if at.n_in(2..=10) => Few,
            Self::Tachelhit => Other,

            // one: n = 0,1 or i = 0 and f = 1
            Self::Sinhala if at.n_is(0) || at.n_is(1) || (at.i == 0 && at.f == 1) => One,
            Self::Sinhala => Other,

            // one: n = 0..1
            Self::ZeroToOne if at.n_in(0..=1) => One,
            Self::ZeroToOne => Other,

            // one: n = 1 / two: n = 2
            Self::OneAndTwo if at.n_is(1) => One,
            Self::OneAndTwo if at.n_is(2) => Two,
            Self::OneAndTwo => Other,

            // one: n = 1 or t != 0 and i = 0,1
            Self::Danish if at.n_is(1) || (at.f != 0 && at.i <= 1) => One,
            Self::Danish => Other,

            // one: v = 0 and i = 1,2,3
            //      or v = 0 and i % 10 != 4,6,9
            //      or v != 0 and f % 10 != 4,6,9
            Self::Filipino
                if (at.v == 0 && (1..=3).contains(&at.i))
                    || (at.v == 0 && !matches!(at.i % 10, 4 | 6 | 9))
                    || (at.v != 0 && !matches!(at.f % 10, 4 | 6 | 9)) =>
            {
                One
            }
            Self::Filipino => Other,
        }
    }
}

/// Every language whose cardinal rules are carried, by its language subtag,
/// **sorted** so [`rules_for`] can bisect it.
///
/// Lower case throughout, because a tag's case is not significant and folding
/// the caller's copy costs nothing while folding this one would cost a second
/// table. `the_locale_table_is_sorted_and_lower_case` is what keeps both true.
///
/// Adding a language is a row here, and an arm above only when its published
/// rule is a shape no arm has. The module doc's gap 3 names the absences that
/// are the second kind.
static RULES: &[(&str, RuleSet)] = &[
    ("af", RuleSet::ExactlyOne),
    ("ak", RuleSet::ZeroToOne),
    ("am", RuleSet::ZeroOrExactlyOne),
    ("ar", RuleSet::Arabic),
    ("ars", RuleSet::Arabic),
    ("as", RuleSet::ZeroOrExactlyOne),
    ("asa", RuleSet::ExactlyOne),
    ("ast", RuleSet::Unit),
    ("az", RuleSet::ExactlyOne),
    ("be", RuleSet::Belarusian),
    ("bem", RuleSet::ExactlyOne),
    ("bez", RuleSet::ExactlyOne),
    ("bg", RuleSet::ExactlyOne),
    ("bh", RuleSet::ZeroToOne),
    ("bn", RuleSet::ZeroOrExactlyOne),
    ("bo", RuleSet::NoDistinction),
    ("br", RuleSet::Breton),
    ("brx", RuleSet::ExactlyOne),
    ("bs", RuleSet::SerboCroatian),
    ("ca", RuleSet::UnitAndMillions),
    ("ce", RuleSet::ExactlyOne),
    ("ceb", RuleSet::Filipino),
    ("cgg", RuleSet::ExactlyOne),
    ("chr", RuleSet::ExactlyOne),
    ("ckb", RuleSet::ExactlyOne),
    ("cs", RuleSet::Czech),
    ("cy", RuleSet::Welsh),
    ("da", RuleSet::Danish),
    ("de", RuleSet::Unit),
    ("doi", RuleSet::ZeroOrExactlyOne),
    ("dsb", RuleSet::Sorbian),
    ("dv", RuleSet::ExactlyOne),
    ("dz", RuleSet::NoDistinction),
    ("ee", RuleSet::ExactlyOne),
    ("el", RuleSet::ExactlyOne),
    ("en", RuleSet::Unit),
    ("eo", RuleSet::ExactlyOne),
    ("es", RuleSet::ExactlyOneAndMillions),
    ("et", RuleSet::Unit),
    ("eu", RuleSet::ExactlyOne),
    ("fa", RuleSet::ZeroOrExactlyOne),
    ("ff", RuleSet::IntegerZeroOrOne),
    ("fi", RuleSet::Unit),
    ("fil", RuleSet::Filipino),
    ("fo", RuleSet::ExactlyOne),
    ("fr", RuleSet::IntegerZeroOrOneAndMillions),
    ("fur", RuleSet::ExactlyOne),
    ("fy", RuleSet::Unit),
    ("ga", RuleSet::Irish),
    ("gd", RuleSet::ScottishGaelic),
    ("gl", RuleSet::Unit),
    ("gsw", RuleSet::ExactlyOne),
    ("gu", RuleSet::ZeroOrExactlyOne),
    ("guw", RuleSet::ZeroToOne),
    ("gv", RuleSet::Manx),
    ("ha", RuleSet::ExactlyOne),
    ("haw", RuleSet::ExactlyOne),
    ("he", RuleSet::Hebrew),
    ("hi", RuleSet::ZeroOrExactlyOne),
    ("hr", RuleSet::SerboCroatian),
    ("hsb", RuleSet::Sorbian),
    ("hu", RuleSet::ExactlyOne),
    ("hy", RuleSet::IntegerZeroOrOne),
    ("ia", RuleSet::Unit),
    ("id", RuleSet::NoDistinction),
    ("ig", RuleSet::NoDistinction),
    ("ii", RuleSet::NoDistinction),
    ("io", RuleSet::Unit),
    ("is", RuleSet::Icelandic),
    ("it", RuleSet::UnitAndMillions),
    ("ja", RuleSet::NoDistinction),
    ("jbo", RuleSet::NoDistinction),
    ("jgo", RuleSet::ExactlyOne),
    ("jmc", RuleSet::ExactlyOne),
    ("jv", RuleSet::NoDistinction),
    ("jw", RuleSet::NoDistinction),
    ("ka", RuleSet::ExactlyOne),
    ("kab", RuleSet::IntegerZeroOrOne),
    ("kaj", RuleSet::ExactlyOne),
    ("kcg", RuleSet::ExactlyOne),
    ("kde", RuleSet::NoDistinction),
    ("kea", RuleSet::NoDistinction),
    ("kk", RuleSet::ExactlyOne),
    ("kkj", RuleSet::ExactlyOne),
    ("kl", RuleSet::ExactlyOne),
    ("km", RuleSet::NoDistinction),
    ("kn", RuleSet::ZeroOrExactlyOne),
    ("ko", RuleSet::NoDistinction),
    ("ks", RuleSet::ExactlyOne),
    ("ksb", RuleSet::ExactlyOne),
    ("ku", RuleSet::ExactlyOne),
    ("kw", RuleSet::Cornish),
    ("ky", RuleSet::ExactlyOne),
    ("lb", RuleSet::ExactlyOne),
    ("lg", RuleSet::ExactlyOne),
    ("lij", RuleSet::Unit),
    ("lkt", RuleSet::NoDistinction),
    ("lo", RuleSet::NoDistinction),
    ("lt", RuleSet::Lithuanian),
    ("lv", RuleSet::Latvian),
    ("mas", RuleSet::ExactlyOne),
    ("mgo", RuleSet::ExactlyOne),
    ("mk", RuleSet::Macedonian),
    ("ml", RuleSet::ExactlyOne),
    ("mn", RuleSet::ExactlyOne),
    ("mo", RuleSet::Romanian),
    ("mr", RuleSet::ExactlyOne),
    ("ms", RuleSet::NoDistinction),
    ("mt", RuleSet::Maltese),
    ("my", RuleSet::NoDistinction),
    ("nah", RuleSet::ExactlyOne),
    ("naq", RuleSet::OneAndTwo),
    ("nb", RuleSet::ExactlyOne),
    ("nd", RuleSet::ExactlyOne),
    ("ne", RuleSet::ExactlyOne),
    ("nl", RuleSet::Unit),
    ("nn", RuleSet::ExactlyOne),
    ("nnh", RuleSet::ExactlyOne),
    ("no", RuleSet::ExactlyOne),
    ("nqo", RuleSet::NoDistinction),
    ("nr", RuleSet::ExactlyOne),
    ("nso", RuleSet::ZeroToOne),
    ("ny", RuleSet::ExactlyOne),
    ("nyn", RuleSet::ExactlyOne),
    ("om", RuleSet::ExactlyOne),
    ("or", RuleSet::ExactlyOne),
    ("os", RuleSet::ExactlyOne),
    ("pap", RuleSet::ExactlyOne),
    ("pcm", RuleSet::ZeroOrExactlyOne),
    ("pl", RuleSet::Polish),
    ("prg", RuleSet::Latvian),
    ("ps", RuleSet::ExactlyOne),
    ("pt", RuleSet::IntegerZeroOrOneAndMillions),
    ("rm", RuleSet::ExactlyOne),
    ("ro", RuleSet::Romanian),
    ("rof", RuleSet::ExactlyOne),
    ("ru", RuleSet::EastSlavic),
    ("rwk", RuleSet::ExactlyOne),
    ("sah", RuleSet::NoDistinction),
    ("saq", RuleSet::ExactlyOne),
    ("sc", RuleSet::Unit),
    ("scn", RuleSet::Unit),
    ("sd", RuleSet::ExactlyOne),
    ("sdh", RuleSet::ExactlyOne),
    ("seh", RuleSet::ExactlyOne),
    ("ses", RuleSet::NoDistinction),
    ("sg", RuleSet::NoDistinction),
    ("sh", RuleSet::SerboCroatian),
    ("shi", RuleSet::Tachelhit),
    ("si", RuleSet::Sinhala),
    ("sk", RuleSet::Czech),
    ("sl", RuleSet::Slovenian),
    ("sn", RuleSet::ExactlyOne),
    ("so", RuleSet::ExactlyOne),
    ("sq", RuleSet::ExactlyOne),
    ("sr", RuleSet::SerboCroatian),
    ("ss", RuleSet::ExactlyOne),
    ("ssy", RuleSet::ExactlyOne),
    ("st", RuleSet::ExactlyOne),
    ("su", RuleSet::NoDistinction),
    ("sv", RuleSet::Unit),
    ("sw", RuleSet::Unit),
    ("syr", RuleSet::ExactlyOne),
    ("ta", RuleSet::ExactlyOne),
    ("te", RuleSet::ExactlyOne),
    ("teo", RuleSet::ExactlyOne),
    ("th", RuleSet::NoDistinction),
    ("tig", RuleSet::ExactlyOne),
    ("tk", RuleSet::ExactlyOne),
    ("tl", RuleSet::Filipino),
    ("tn", RuleSet::ExactlyOne),
    ("to", RuleSet::NoDistinction),
    ("tr", RuleSet::ExactlyOne),
    ("ts", RuleSet::ExactlyOne),
    ("tzm", RuleSet::Tamazight),
    ("ug", RuleSet::ExactlyOne),
    ("uk", RuleSet::EastSlavic),
    ("ur", RuleSet::Unit),
    ("uz", RuleSet::ExactlyOne),
    ("ve", RuleSet::ExactlyOne),
    ("vi", RuleSet::NoDistinction),
    ("vo", RuleSet::ExactlyOne),
    ("vun", RuleSet::ExactlyOne),
    ("wa", RuleSet::ZeroToOne),
    ("wae", RuleSet::ExactlyOne),
    ("wo", RuleSet::NoDistinction),
    ("xh", RuleSet::ExactlyOne),
    ("xog", RuleSet::ExactlyOne),
    ("yi", RuleSet::Unit),
    ("yo", RuleSet::NoDistinction),
    ("yue", RuleSet::NoDistinction),
    ("zh", RuleSet::NoDistinction),
    ("zu", RuleSet::ZeroOrExactlyOne),
];

/// One CLDR **ordinal** rule set, named for a language that uses it.
///
/// A second enum rather than more [`RuleSet`] arms because the two tables are
/// separate in CLDR and disagree for the same language: English marks `2nd` and
/// makes no cardinal distinction at 2, and Russian is the reverse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrdinalSet {
    /// No marked ordinal form — the bucket the module doc calls the default,
    /// and what a language absent from [`ORDINALS`] answers.
    Unmarked,
    /// English: `1st`, `2nd`, `3rd`, and `4th` for everything else.
    English,
    /// Swedish, whose first two forms are one and which drops the teens.
    Swedish,
    /// Ukrainian, whose one marked form is the third.
    Ukrainian,
    /// Belarusian, whose one marked form covers the second and the third.
    Belarusian,
    /// Italian, Sardinian and Sicilian: four counts and nothing else.
    Italian,
    /// The first and nothing else — French, Irish, Armenian, Lao, Malay,
    /// Romanian, Vietnamese and the Filipino pair, which is the widest group
    /// CLDR publishes one ordinal rule for.
    FirstOnly,
    /// Hungarian, which marks the first and the fifth.
    Hungarian,
    /// Nepali, which marks the first four.
    Nepali,
    /// Catalan, whose `one` is two disjoint counts.
    Catalan,
    /// Marathi, which is [`Self::Hindi`] without the sixth.
    Marathi,
    /// Hindi and Gujarati.
    Hindi,
    /// Assamese and Bengali, whose `one` is six counts.
    Bengali,
    /// Odia: [`Self::Bengali`] without the tenth.
    Odia,
    /// Azerbaijani, the widest ordinal rule carried here.
    Azerbaijani,
    /// Albanian, whose `many` is the fourth of every ten but the fourteenth.
    Albanian,
    /// Kazakh, whose one marked form is a set of last digits.
    Kazakh,
    /// Welsh, which marks all five and is the only ordinal set with a `zero`.
    Welsh,
    /// Cornish, whose `one` is the first four counts and reopens in bands of
    /// every hundred, and whose `many` is the fifth of each.
    Cornish,
    /// Macedonian, which reads `i` where the sets above read `n`.
    Macedonian,
    /// Georgian, whose `many` is a range of hundreds-remainders.
    Georgian,
    /// Turkmen.
    Turkmen,
}

impl OrdinalSet {
    /// CLDR's own ordinal rule for this set, in the order the categories are
    /// published in — [`RuleSet::select`]'s shape, over the second table.
    fn select(self, at: Operands) -> Category {
        use Category::{Few, Many, One, Other, Two, Zero};
        match self {
            Self::Unmarked => Other,

            // one: n % 10 = 1 and n % 100 != 11
            // two: n % 10 = 2 and n % 100 != 12
            // few: n % 10 = 3 and n % 100 != 13
            Self::English if at.n_mod_is(10, 1) && !at.n_mod_is(100, 11) => One,
            Self::English if at.n_mod_is(10, 2) && !at.n_mod_is(100, 12) => Two,
            Self::English if at.n_mod_is(10, 3) && !at.n_mod_is(100, 13) => Few,
            Self::English => Other,

            // one: n % 10 = 1,2 and n % 100 != 11,12
            Self::Swedish if at.n_mod_any(10, &[1, 2]) && !at.n_mod_any(100, &[11, 12]) => One,
            Self::Swedish => Other,

            // few: n % 10 = 3 and n % 100 != 13
            Self::Ukrainian if at.n_mod_is(10, 3) && !at.n_mod_is(100, 13) => Few,
            Self::Ukrainian => Other,

            // few: n % 10 = 2,3 and n % 100 != 12,13
            Self::Belarusian if at.n_mod_any(10, &[2, 3]) && !at.n_mod_any(100, &[12, 13]) => Few,
            Self::Belarusian => Other,

            // many: n = 11,8,80,800
            Self::Italian if at.n_any(&[8, 11, 80, 800]) => Many,
            Self::Italian => Other,

            // one: n = 1
            Self::FirstOnly if at.n_is(1) => One,
            Self::FirstOnly => Other,

            // one: n = 1,5
            Self::Hungarian if at.n_any(&[1, 5]) => One,
            Self::Hungarian => Other,

            // one: n = 1..4
            Self::Nepali if at.n_in(1..=4) => One,
            Self::Nepali => Other,

            // one: n = 1,3 / two: n = 2 / few: n = 4
            Self::Catalan if at.n_any(&[1, 3]) => One,
            Self::Catalan if at.n_is(2) => Two,
            Self::Catalan if at.n_is(4) => Few,
            Self::Catalan => Other,

            // one: n = 1 / two: n = 2,3 / few: n = 4
            Self::Marathi if at.n_is(1) => One,
            Self::Marathi if at.n_any(&[2, 3]) => Two,
            Self::Marathi if at.n_is(4) => Few,
            Self::Marathi => Other,

            // one: n = 1 / two: n = 2,3 / few: n = 4 / many: n = 6
            Self::Hindi if at.n_is(1) => One,
            Self::Hindi if at.n_any(&[2, 3]) => Two,
            Self::Hindi if at.n_is(4) => Few,
            Self::Hindi if at.n_is(6) => Many,
            Self::Hindi => Other,

            // one: n = 1,5,7,8,9,10 / two: n = 2,3 / few: n = 4 / many: n = 6
            Self::Bengali if at.n_any(&[1, 5, 7, 8, 9, 10]) => One,
            Self::Bengali if at.n_any(&[2, 3]) => Two,
            Self::Bengali if at.n_is(4) => Few,
            Self::Bengali if at.n_is(6) => Many,
            Self::Bengali => Other,

            // one: n = 1,5,7..9 / two: n = 2,3 / few: n = 4 / many: n = 6
            Self::Odia if at.n_any(&[1, 5, 7, 8, 9]) => One,
            Self::Odia if at.n_any(&[2, 3]) => Two,
            Self::Odia if at.n_is(4) => Few,
            Self::Odia if at.n_is(6) => Many,
            Self::Odia => Other,

            // one:  n % 10 = 1,2,5,7,8 or n % 100 = 20,50,70,80
            // few:  n % 10 = 3,4 or n % 1000 = 100,200,300,400,500,600,700,800,900
            // many: n = 0 or n % 10 = 6 or n % 100 = 40,60,90
            Self::Azerbaijani
                if at.n_mod_any(10, &[1, 2, 5, 7, 8]) || at.n_mod_any(100, &[20, 50, 70, 80]) =>
            {
                One
            }
            Self::Azerbaijani
                if at.n_mod_any(10, &[3, 4])
                    || at.n_mod_any(1_000, &[100, 200, 300, 400, 500, 600, 700, 800, 900]) =>
            {
                Few
            }
            Self::Azerbaijani
                if at.n_is(0) || at.n_mod_is(10, 6) || at.n_mod_any(100, &[40, 60, 90]) =>
            {
                Many
            }
            Self::Azerbaijani => Other,

            // one: n = 1 / many: n % 10 = 4 and n % 100 != 14
            Self::Albanian if at.n_is(1) => One,
            Self::Albanian if at.n_mod_is(10, 4) && !at.n_mod_is(100, 14) => Many,
            Self::Albanian => Other,

            // many: n % 10 = 6 or n % 10 = 9 or n % 10 = 0 and n != 0
            Self::Kazakh if at.n_mod_any(10, &[6, 9]) || (at.n_mod_is(10, 0) && !at.n_is(0)) => {
                Many
            }
            Self::Kazakh => Other,

            // zero: n = 0,7,8,9 / one: n = 1 / two: n = 2 / few: n = 3,4
            // many: n = 5,6
            Self::Welsh if at.n_any(&[0, 7, 8, 9]) => Zero,
            Self::Welsh if at.n_is(1) => One,
            Self::Welsh if at.n_is(2) => Two,
            Self::Welsh if at.n_any(&[3, 4]) => Few,
            Self::Welsh if at.n_any(&[5, 6]) => Many,
            Self::Welsh => Other,

            // one:  n = 1..4 or n % 100 = 1..4,21..24,41..44,61..64,81..84
            // many: n = 5 or n % 100 = 5
            Self::Cornish
                if at.n_in(1..=4)
                    || [1, 21, 41, 61, 81]
                        .into_iter()
                        .any(|base| at.n_mod_in(100, base..=base + 3)) =>
            {
                One
            }
            Self::Cornish if at.n_is(5) || at.n_mod_is(100, 5) => Many,
            Self::Cornish => Other,

            // one:  i % 10 = 1 and i % 100 != 11
            // two:  i % 10 = 2 and i % 100 != 12
            // many: i % 10 = 7,8 and i % 100 != 17,18
            Self::Macedonian if at.i % 10 == 1 && at.i % 100 != 11 => One,
            Self::Macedonian if at.i % 10 == 2 && at.i % 100 != 12 => Two,
            Self::Macedonian if matches!(at.i % 10, 7 | 8) && !matches!(at.i % 100, 17 | 18) => {
                Many
            }
            Self::Macedonian => Other,

            // one: i = 1 / many: i = 0 or i % 100 = 2..20,40,60,80
            Self::Georgian if at.i == 1 => One,
            Self::Georgian
                if at.i == 0
                    || (2..=20).contains(&(at.i % 100))
                    || matches!(at.i % 100, 40 | 60 | 80) =>
            {
                Many
            }
            Self::Georgian => Other,

            // few: n % 10 = 6,9 or n = 10
            Self::Turkmen if at.n_mod_any(10, &[6, 9]) || at.n_is(10) => Few,
            Self::Turkmen => Other,
        }
    }
}

/// Every language that marks an ordinal form, by its language subtag,
/// **sorted** so [`ordinal_rules_for`] can bisect it — [`RULES`]'s two
/// properties, held by the same test.
///
/// Shorter than [`RULES`] on purpose: this is the set of languages that differ
/// from `Other`, which is how CLDR's own ordinal data is written, and it is
/// **every** one of them the cardinal roster carries. The module doc's ordinal
/// section is the home of why the absence answers rather than throws;
/// `an_ordinal_category_is_answered_for_every_language_with_a_published_table`
/// holds every row here to a row there, and
/// `every_language_cldr_gives_an_ordinal_rule_is_on_the_ordinal_roster` runs
/// the other direction, so a published rule this table omits fails there
/// rather than answering `Other` in a catalog.
static ORDINALS: &[(&str, OrdinalSet)] = &[
    ("as", OrdinalSet::Bengali),
    ("az", OrdinalSet::Azerbaijani),
    ("be", OrdinalSet::Belarusian),
    ("bn", OrdinalSet::Bengali),
    ("ca", OrdinalSet::Catalan),
    ("cy", OrdinalSet::Welsh),
    ("en", OrdinalSet::English),
    ("fil", OrdinalSet::FirstOnly),
    ("fr", OrdinalSet::FirstOnly),
    ("ga", OrdinalSet::FirstOnly),
    ("gu", OrdinalSet::Hindi),
    ("hi", OrdinalSet::Hindi),
    ("hu", OrdinalSet::Hungarian),
    ("hy", OrdinalSet::FirstOnly),
    ("it", OrdinalSet::Italian),
    ("ka", OrdinalSet::Georgian),
    ("kk", OrdinalSet::Kazakh),
    ("kw", OrdinalSet::Cornish),
    ("lo", OrdinalSet::FirstOnly),
    ("mk", OrdinalSet::Macedonian),
    ("mo", OrdinalSet::FirstOnly),
    ("mr", OrdinalSet::Marathi),
    ("ms", OrdinalSet::FirstOnly),
    ("ne", OrdinalSet::Nepali),
    ("or", OrdinalSet::Odia),
    ("ro", OrdinalSet::FirstOnly),
    ("sc", OrdinalSet::Italian),
    ("scn", OrdinalSet::Italian),
    ("sq", OrdinalSet::Albanian),
    ("sv", OrdinalSet::Swedish),
    ("tk", OrdinalSet::Turkmen),
    ("tl", OrdinalSet::FirstOnly),
    ("uk", OrdinalSet::Ukrainian),
    ("vi", OrdinalSet::FirstOnly),
];

/// The language subtag of a BCP 47 tag — everything before the first separator.
///
/// A tag's other subtags are deliberately dropped rather than tried first:
/// CLDR's cardinal rules are language-level data, so `pt-BR` and `pt-PT` share
/// one rule set and looking for a region-specific one would be a lookup that
/// can never hit.
fn subtag_of<'tag>(tag: &'tag str, member: &str) -> Result<&'tag str, Fault> {
    let subtag = tag.split(['-', '_']).next().unwrap_or("");
    if subtag.is_empty() || !subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{member} read no language subtag from the locale `{tag}`: a tag begins with \
                 letters"
            ),
        ));
    }
    Ok(subtag)
}

/// `carried.cmp(given)` with `given`'s ASCII letters folded down, so [`RULES`]
/// can stay lower case and `EN` still find `en` without an allocation per
/// probe.
fn compare_folded(carried: &str, given: &str) -> Ordering {
    carried
        .bytes()
        .cmp(given.bytes().map(|byte| byte.to_ascii_lowercase()))
}

/// The rules for a language subtag, or the refusal the module doc argues for.
fn rules_for(subtag: &str, member: &str) -> Result<RuleSet, Fault> {
    RULES
        .binary_search_by(|(carried, _)| compare_folded(carried, subtag))
        .map(|index| RULES[index].1)
        .map_err(|_| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!(
                    "{member} carries no plural rules for the language \
                     `{subtag}`: a language is never given another one's rules, because a \
                     catalog written against the wrong forms reads correctly for one count and \
                     wrongly for the rest"
                ),
            )
        })
}

/// The operands of a `float`, read off the digits it prints.
///
/// The module doc owns why that is the right reading. `Display` for `f64` is
/// the shortest representation that round-trips and never uses an exponent, so
/// splitting it at the point is the whole of the parse — and the two counts it
/// refuses are the two it cannot hold: an integer part past `u128`, and a
/// denormal's several hundred fraction digits.
fn operands_of_float(value: f64, member: &str) -> Result<Operands, Fault> {
    if !value.is_finite() {
        return Err(Fault::thrown(format!(
            "{member} has no category for the count `{value}`: it is not a \
             finite number"
        )));
    }
    let printed = format!("{}", value.abs());
    let (whole, fraction) = printed.split_once('.').unwrap_or((printed.as_str(), ""));
    let too_wide = || {
        Fault::thrown(format!(
            "{member} cannot classify the count `{value}`: it prints more \
             digits than the plural rules are evaluated over"
        ))
    };
    Ok(Operands {
        i: whole.parse().map_err(|_| too_wide())?,
        v: u32::try_from(fraction.len()).map_err(|_| too_wide())?,
        f: if fraction.is_empty() {
            0
        } else {
            fraction.parse().map_err(|_| too_wide())?
        },
    })
}

/// Reads argument `slot` as a count of the spec's `int|float|decimal` union
/// and derives CLDR's operands from it.
///
/// A `uint` is accepted for [`crate::math`]'s reason: it reaches a union
/// parameter through the same tagged slot, and refusing it would be a `FATAL`
/// for a value with an exact answer.
fn operands_at(args: &[Value], slot: usize, member: &str) -> Result<Operands, Fault> {
    if let Some(int) = args[slot].as_int() {
        return Ok(Operands {
            i: u128::from(int.unsigned_abs()),
            v: 0,
            f: 0,
        });
    }
    if let Some(uint) = args[slot].as_uint() {
        return Ok(Operands {
            i: u128::from(uint),
            v: 0,
            f: 0,
        });
    }
    if let Some(exact) = args[slot].as_decimal() {
        let (i, f) = match 10u128.checked_pow(u32::from(exact.scale())) {
            Some(divisor) => (exact.mantissa() / divisor, exact.mantissa() % divisor),
            // A scale past `u128`'s reach puts every digit in the fraction,
            // which is the same answer the division would have given.
            None => (0, exact.mantissa()),
        };
        return Ok(Operands {
            i,
            v: u32::from(exact.scale()),
            f,
        });
    }
    if let Some(float) = args[slot].as_float() {
        return operands_of_float(float, member);
    }
    // A fatal rather than a throw, because nothing may catch a broken ABI: the
    // row's parameter is `CoreTy::Union(NUMBER)`, so `E0401` refuses anything
    // outside `int|uint|float|decimal` before a single instruction of this
    // body runs, which makes the message below unreachable from source.
    Err(Fault::fatal(format!(
        "{member} expected a number at argument {slot}, got tag {}",
        args[slot].tag_byte()
    )))
}

nvs_runtime::nvs_helper! {
    /// `Core\Cldr::pluralCategory(int|float|decimal $count, string $locale):
    /// Cldr\PluralCategory` — `rule:programs/framework-core-half`'s row, and the whole of the class.
    ///
    /// The count is parameter 1 because it is what the member classifies and
    /// the locale is the rule it is classified under — the same order
    /// `Core\Time::parse(string $text, string $pattern)` writes, where the
    /// text is what is read and the pattern says how (`rule:core-api/shape-rules` R1).
    ///
    /// The locale is `Qual::Neutral` rather than a sink: it is a lookup key,
    /// nothing it names is executed, and the answer is an ordinal that carries
    /// no qualifier at all. So a tag negotiated out of an `Accept-Language`
    /// header arrives here still `tainted` and is fine, which is the point —
    /// requiring a launderer for it would be a ceremony with no sink behind it.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (`rule:enums/closed-integer-type`).
    fn nvs_core_cldr_plural_category(_ctx, args: [2]) {
        let operands = operands_at(args, 0, PLURAL_MEMBER)?;
        // Unreachable from source for the reason `operands_at`'s own fatal
        // states: the row's second parameter is `CoreTy::Text`, so `E0401`
        // refuses anything that is not a `string` before this body runs.
        let tag = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cldr::pluralCategory expected a `string` locale, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let rules = rules_for(subtag_of(tag, PLURAL_MEMBER)?, PLURAL_MEMBER)?;
        Ok(Value::int(rules.select(operands).ordinal()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cldr::ordinalCategory(int|float|decimal $count, string $locale):
    /// Cldr\PluralCategory` — the second table, over the first member's shape.
    ///
    /// Everything about the signature is `pluralCategory`'s and for its
    /// reasons: the count first, the locale `Qual::Neutral`, and the same six
    /// categories answered as an ordinal. What differs is only which table is
    /// read, which is why this is a second member rather than a third argument
    /// — a `kind:` option would make the two rosters' different treatment of an
    /// absent language a runtime surprise instead of a member you did not call.
    fn nvs_core_cldr_ordinal_category(_ctx, args: [2]) {
        let operands = operands_at(args, 0, ORDINAL_MEMBER)?;
        // Unreachable from source, for `pluralCategory`'s own reason.
        let tag = args[1].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{ORDINAL_MEMBER} expected a `string` locale, got tag {}",
                args[1].tag_byte()
            ))
        })?;
        let rules = ordinal_rules_for(subtag_of(tag, ORDINAL_MEMBER)?, ORDINAL_MEMBER)?;
        Ok(Value::int(rules.select(operands).ordinal()))
    }
}

/// The ordinal rules for a language subtag: its own row, `Unmarked` for a
/// language [`RULES`] carries and this table does not, and [`rules_for`]'s
/// refusal for one neither carries.
///
/// The module doc's ordinal section is the home of why the middle case answers
/// rather than throws.
fn ordinal_rules_for(subtag: &str, member: &str) -> Result<OrdinalSet, Fault> {
    if let Ok(index) = ORDINALS.binary_search_by(|(carried, _)| compare_folded(carried, subtag)) {
        return Ok(ORDINALS[index].1);
    }
    rules_for(subtag, member).map(|_| OrdinalSet::Unmarked)
}

pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_cldr_plural_category" => (nvs_core_cldr_plural_category as *const ()).cast(),
        "nvs_core_cldr_ordinal_category" => (nvs_core_cldr_ordinal_category as *const ()).cast(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The zone every test here formats in, so nothing depends on the host's.
    fn utc() -> TimeZone {
        TimeZone::UTC
    }

    /// A `Zoned` at the civil time given, in UTC.
    fn at(y: i16, mo: i8, d: i8, h: i8, mi: i8, s: i8, ns: i32) -> Zoned {
        utc()
            .to_zoned(civil::DateTime::new(y, mo, d, h, mi, s, ns).unwrap())
            .unwrap()
    }

    /// A pattern written as a literal is compiled once per core; one the
    /// program assembled is compiled at the call and left out of the cache.
    #[test]
    fn a_literal_cldr_pattern_is_prepared_at_compile_time_and_not_per_call() {
        // The compile-time half: the checker reaches this module through
        // `validate`, and one word is all it hands the call afterwards.
        validate("yyyy-MM-dd HH:mm:ss").expect("a pattern");
        assert!(prepared(PREPARED_PATTERN));
        // The zero word and a word another grammar minted are one answer, so a
        // word that reached the wrong helper costs a compile rather than a
        // wrong pattern.
        assert!(!prepared(PREPARED_NONE));
        assert!(!prepared(crate::regex::PREPARED_LINEAR));

        let first = compiled("yyyy-MM-dd HH:mm:ss", true).expect("a pattern");
        let again = compiled("yyyy-MM-dd HH:mm:ss", true).expect("a pattern");
        assert!(
            Rc::ptr_eq(&first, &again),
            "a prepared pattern was compiled a second time"
        );

        let built = compiled("EEEE, d MMMM yyyy", false).expect("a pattern");
        let rebuilt = compiled("EEEE, d MMMM yyyy", false).expect("a pattern");
        assert!(
            !Rc::ptr_eq(&built, &rebuilt),
            "a pattern the program assembled was keyed on this core's cache"
        );
    }

    /// The cache is bounded and cleared wholesale when it fills, which is what
    /// keeps a core's footprint O(written patterns) rather than O(calls).
    #[test]
    fn the_prepared_pattern_cache_never_grows_past_its_capacity() {
        for nth in 0..=CACHE_CAPACITY {
            compiled(&format!("'bounded-{nth}'"), true).expect("a pattern");
        }
        CACHE.with_borrow(|cache| assert!(cache.len() <= CACHE_CAPACITY, "{}", cache.len()));
    }

    /// § 4's own two example patterns, which is the shape every other test
    /// here varies.
    #[test]
    fn the_two_patterns_the_spec_writes_render() {
        let value = at(2024, 3, 1, 14, 5, 9, 0);
        assert_eq!(
            render(&compile("yyyy-MM-dd HH:mm:ss").unwrap(), &value),
            "2024-03-01 14:05:09"
        );
        assert_eq!(
            render(&compile("EEEE, d MMMM yyyy").unwrap(), &value),
            "Friday, 1 March 2024"
        );
    }

    /// Every count of every letter in the table, so a change to one row cannot
    /// pass by agreeing with itself.
    #[test]
    fn each_count_picks_its_own_rendering() {
        let value = at(2024, 3, 1, 14, 5, 9, 123_456_789);
        for (pattern, want) in [
            ("y", "2024"),
            ("yy", "24"),
            ("yyyyy", "02024"),
            ("M/MM/MMM/MMMM/MMMMM", "3/03/Mar/March/M"),
            ("d/dd", "1/01"),
            ("D/DDD", "61/061"),
            ("E/EEEE/EEEEE/EEEEEE", "Fri/Friday/F/Fr"),
            ("a", "PM"),
            ("h/H/K/k", "2/14/2/14"),
            ("m/mm/s/ss", "5/05/9/09"),
            ("S/SSS/SSSSSS", "1/123/123456"),
            ("X/XX/XXX", "Z/Z/Z"),
            ("x/xx/xxx", "+00/+0000/+00:00"),
            ("VV", "UTC"),
        ] {
            assert_eq!(
                render(&compile(pattern).unwrap(), &value),
                want,
                "{pattern}"
            );
        }
    }

    /// Midnight is where the four hour letters disagree, so it gets its own
    /// case.
    #[test]
    fn the_four_hour_letters_disagree_at_midnight() {
        let value = at(2024, 3, 1, 0, 0, 0, 0);
        assert_eq!(
            render(&compile("h H K k a").unwrap(), &value),
            "12 0 0 24 AM"
        );
    }

    /// CLDR's quoting: a `'…'` run is literal, `''` is one apostrophe, and an
    /// unquoted letter outside the table is refused rather than emitted.
    #[test]
    fn a_quoted_run_is_literal_and_an_unknown_letter_is_refused() {
        let value = at(2024, 3, 1, 14, 0, 0, 0);
        assert_eq!(
            render(&compile("yyyy 'at' HH'h'").unwrap(), &value),
            "2024 at 14h"
        );
        assert_eq!(render(&compile("''yy''").unwrap(), &value), "'24'");
        assert!(compile("yyyy j").unwrap_err().contains("`j` is not"));
        assert!(compile("yyyy 'unclosed").unwrap_err().contains("closes"));
        assert!(compile("V").unwrap_err().contains("`VV`"));
    }

    /// A non-ASCII literal survives, which is the one place [`compile`]'s
    /// byte-wise scan could have split a character.
    #[test]
    fn a_non_ascii_literal_survives() {
        let value = at(2024, 3, 1, 14, 0, 0, 0);
        assert_eq!(render(&compile("d–M–yyyy").unwrap(), &value), "1–3–2024");
    }

    /// Reading is the inverse of rendering for the patterns a program writes.
    #[test]
    fn what_renders_reads_back() {
        let zone = TimeZone::get("Europe/Berlin").unwrap();
        for pattern in ["yyyy-MM-dd HH:mm:ss", "d MMMM yyyy HH:mm", "yy/M/d h:mm a"] {
            let pieces = compile(pattern).unwrap();
            let value = utc()
                .to_zoned(civil::DateTime::new(2024, 7, 15, 14, 5, 0, 0).unwrap())
                .unwrap()
                .with_time_zone(zone.clone());
            let text = render(&pieces, &value);
            let read = read(&pieces, &text, &zone).unwrap();
            assert_eq!(render(&pieces, &read), text, "{pattern}");
        }
    }

    /// Every way a read can fail says which one it was.
    #[test]
    fn a_read_that_does_not_match_says_why() {
        let pieces = compile("yyyy-MM-dd").unwrap();
        let zone = utc();
        assert!(
            read(&pieces, "2024/03/01", &zone)
                .unwrap_err()
                .contains("expected `-`")
        );
        assert!(
            read(&pieces, "2024-03-01x", &zone)
                .unwrap_err()
                .contains("trailing")
        );
        assert!(
            read(&pieces, "2024-03-", &zone)
                .unwrap_err()
                .contains("digits of day")
        );
        assert!(
            read(&pieces, "2024-13-01", &zone)
                .unwrap_err()
                .contains("month")
        );
        assert!(
            read(&compile("yyyy X").unwrap(), "2024 Z", &zone)
                .unwrap_err()
                .contains("third argument")
        );
    }

    /// A month name is matched longest-first, so the full name is not read as
    /// the short one with trailing text.
    #[test]
    fn a_name_is_matched_longest_first() {
        let pieces = compile("MMM d yyyy").unwrap();
        let zone = utc();
        assert_eq!(
            render(&pieces, &read(&pieces, "June 3 2024", &zone).unwrap()),
            "Jun 3 2024"
        );
        assert_eq!(
            render(&pieces, &read(&pieces, "jun 3 2024", &zone).unwrap()),
            "Jun 3 2024"
        );
    }

    /// A two-digit year lands in the 1969–2068 window at both ends of it.
    #[test]
    fn a_two_digit_year_lands_in_its_window() {
        let pieces = compile("yy-MM-dd").unwrap();
        let zone = utc();
        assert_eq!(read(&pieces, "68-01-01", &zone).unwrap().year(), 2068);
        assert_eq!(read(&pieces, "69-01-01", &zone).unwrap().year(), 1969);
    }

    /// The module doc's gap 2 used to name eight letters this subset refused.
    /// It names none of them now: each renders, each reads back, and the two
    /// that carry something a civil date does not already hold — the era and
    /// the signed proleptic year — reach the value rather than being consumed
    /// and dropped.
    #[test]
    fn the_eight_refused_pattern_letters_format_and_parse() {
        let zone = utc();
        // A Friday, in ISO week 9, in a month that opens on a three-day week.
        let value = at(2024, 3, 1, 14, 0, 0, 0);
        let show = |pattern: &str| render(&compile(pattern).unwrap(), &value);

        assert_eq!(show("G GGGG GGGGG"), "AD Anno Domini A");
        assert_eq!(show("Q QQ QQQ QQQQ QQQQQ"), "1 01 Q1 1st quarter 1");
        assert_eq!(show("w ww"), "9 09");
        assert_eq!(show("L LL LLL LLLL LLLLL"), "3 03 Mar March M");
        assert_eq!(show("c cc ccc cccc ccccc cccccc"), "5 05 Fri Friday F Fr");
        assert_eq!(show("F"), "1", "1 March 2024 is the month's first Friday");
        assert_eq!(show("u uuuu"), "2024 2024");

        // `W`'s bound, on both sides: March 2024 opens on a Friday, so its
        // first three days are a week too short to be week 1 and the Monday
        // after them starts it. A rule without the four-day minimum answers 1
        // and 2 for these two and reads plausibly.
        assert_eq!(show("W"), "0");
        assert_eq!(
            render(&compile("W").unwrap(), &at(2024, 3, 4, 0, 0, 0, 0)),
            "1"
        );

        // `y` and `u` render the same signed proleptic year — the module doc's
        // own deviation from CLDR, and what keeps `yyyy-MM-dd` sortable over
        // the whole range a `Core\Time\Date` accepts. `G` renders that sign as
        // a word, and on the way back it is read and discarded, so it can
        // neither complete nor contradict the year beside it.
        let bc = at(-43, 3, 15, 12, 0, 0, 0);
        assert_eq!(render(&compile("yyyy G").unwrap(), &bc), "-0043 BC");
        assert_eq!(render(&compile("uuuu G").unwrap(), &bc), "-0043 BC");
        assert_eq!(render(&compile("yyyy G").unwrap(), &value), "2024 AD");
        assert_eq!(
            read(&compile("uuuu G").unwrap(), "-0043 BC", &zone)
                .unwrap()
                .year(),
            -43,
            "`u` is the letter that reads a sign back"
        );
        assert_eq!(
            read(&compile("uuuu GGGG").unwrap(), "-0043 Anno Domini", &zone)
                .unwrap()
                .year(),
            -43,
            "and the era beside it changes nothing, because it is discarded"
        );
        assert!(
            read(&compile("yyyy").unwrap(), "-0043", &zone).is_err(),
            "`y` reads digits, which is the difference the two letters have"
        );

        // The standalone month reaches the value the way `M` does, and the
        // five derived letters are read and discarded — so a pattern carrying
        // all of them reads back the date its `y`, `M` and `d` named.
        assert_eq!(
            read(&compile("LLLL d yyyy").unwrap(), "March 4 2024", &zone)
                .unwrap()
                .month(),
            3
        );
        let every = compile("yyyy-MM-dd QQQ 'w'w 'W'W F ccc").unwrap();
        let written = render(&every, &value);
        assert_eq!(written, "2024-03-01 Q1 w9 W0 1 Fri");
        let back = read(&every, &written, &zone).unwrap();
        assert_eq!((back.year(), back.month(), back.day()), (2024, 3, 1));

        // All eight are calendar fields, so the three narrowing guards place
        // them the same way they place `y` and `M`: a `Date` renders them, a
        // `TimeOfDay` does not, and a parse pattern may name them.
        for letter in ["G", "Q", "w", "W", "L", "c", "F", "u"] {
            let pieces = compile(letter).unwrap();
            assert!(date_fields_only(&pieces).is_ok(), "`{letter}` on a date");
            assert!(time_fields_only(&pieces).is_err(), "`{letter}` on a clock");
            assert!(civil_fields_only(&pieces).is_ok(), "`{letter}` in a parse");
        }
    }

    /// A `Zoned` at the civil time given, at a fixed offset of `hours` and
    /// `minutes` — the only way the zone letters render anything but `GMT`.
    fn at_offset(minutes: i32, h: i8, mi: i8) -> Zoned {
        let offset =
            Offset::from_seconds(minutes * 60).expect("a whole-minute offset inside a day");
        TimeZone::fixed(offset)
            .to_zoned(civil::DateTime::new(2024, 3, 1, h, mi, 0, 0).unwrap())
            .expect("a fixed offset has no gap to fall in")
    }

    /// The ten letters gap 2 used to refuse beside `Y` and `e`, each rendering
    /// what ICU renders under the Gregorian calendar: the two year letters
    /// resolve onto the two this subset already writes, the two day-period
    /// letters read English's periods, and the four zone letters render the
    /// localized GMT format ICU falls back to where no name is carried.
    #[test]
    fn every_pattern_letter_once_refused_now_formats_as_icu_does() {
        let zone = utc();
        let value = at(2024, 3, 1, 14, 5, 9, 123_000_000);
        let show = |pattern: &str| render(&compile(pattern).unwrap(), &value);

        // `U` falls back to the numeric year, two-digit window and all, and
        // `r` is the related Gregorian year, which has none — the difference
        // `y` and `u` already carry.
        assert_eq!(show("U UU UUUUU"), "2024 24 02024");
        assert_eq!(show("r rr rrrrr"), "2024 2024 02024");
        assert_eq!(show("A AAAAAAAAA"), "50709123 050709123");
        assert_eq!(show("g"), "60370", "1 March 2024 is MJD 60370");
        assert_eq!(show("b bbbb bbbbb"), "PM PM p");
        assert_eq!(
            show("B BBBB BBBBB"),
            "in the afternoon in the afternoon in the afternoon"
        );

        // Midnight and noon are the two instants with a name of their own, and
        // one minute past either is not one of them — the bound on both sides.
        for (h, mi, want) in [
            (0, 0, "midnight midnight mi"),
            (0, 1, "AM at night a"),
            (12, 0, "noon noon n"),
            (12, 1, "PM in the afternoon p"),
        ] {
            assert_eq!(
                render(&compile("b B bbbbb").unwrap(), &at(2024, 3, 1, h, mi, 0, 0)),
                want,
                "{h}:{mi:02}"
            );
        }
        assert_eq!(
            render(&compile("b B bbbbb").unwrap(), &at(2024, 3, 1, 0, 0, 1, 0)),
            "AM at night a",
            "a second past midnight is no longer the instant midnight names"
        );
        assert_eq!(
            render(&compile("b B").unwrap(), &at(2024, 3, 1, 12, 0, 0, 999)),
            "noon noon",
            "and the fraction is not read, which is where ICU draws the line"
        );
        for (hour, want) in [
            (5, "at night"),
            (6, "in the morning"),
            (18, "in the evening"),
            (21, "at night"),
        ] {
            assert_eq!(
                render(&compile("B").unwrap(), &at(2024, 3, 1, hour, 30, 0, 0)),
                want,
                "{hour}:30"
            );
        }

        // Every zone letter at a zero offset is the `GMT` the localized format
        // writes for one, except the two ISO spellings that never say `GMT`.
        assert_eq!(
            show("z zzzz v vvvv O OOOO ZZZZ"),
            "GMT GMT GMT GMT GMT GMT GMT"
        );
        assert_eq!(show("Z ZZ ZZZ ZZZZZ"), "+0000 +0000 +0000 Z");

        // And away from it, where the short and long forms differ. A whole
        // hour writes no minutes in the short form and does in the long one.
        let west = at_offset(-8 * 60, 14, 5);
        let show_west = |pattern: &str| render(&compile(pattern).unwrap(), &west);
        assert_eq!(show_west("z zzz zzzz"), "GMT-8 GMT-8 GMT-08:00");
        assert_eq!(show_west("v vvvv"), "GMT-8 GMT-08:00");
        assert_eq!(show_west("O OOOO"), "GMT-8 GMT-08:00");
        assert_eq!(
            show_west("Z ZZZ ZZZZ ZZZZZ"),
            "-0800 -0800 GMT-08:00 -08:00"
        );
        let east = at_offset(5 * 60 + 30, 14, 5);
        let show_east = |pattern: &str| render(&compile(pattern).unwrap(), &east);
        assert_eq!(show_east("O OOOO"), "GMT+5:30 GMT+05:30");
        assert_eq!(show_east("z zzzz"), "GMT+5:30 GMT+05:30");
        assert_eq!(show_east("Z ZZZZZ"), "+0530 +05:30");

        // A count CLDR does not give these letters is refused, as `V`'s is,
        // rather than rendered as the nearest one it does give.
        for pattern in ["vv", "vvv", "OO", "ZZZZZZ", "V"] {
            assert!(compile(pattern).is_err(), "`{pattern}`");
        }

        // The six non-zonal letters reach the value on the way back: `A` and
        // `g` carry a whole time of day and a whole date by themselves, and a
        // day period completes the 12-hour clock beside it.
        let back = read(&compile("g A").unwrap(), "60370 50709123", &zone).unwrap();
        assert_eq!(
            (back.year(), back.month(), back.day()),
            (2024, 3, 1),
            "`g` is the date by itself"
        );
        assert_eq!(
            (
                back.hour(),
                back.minute(),
                back.second(),
                back.millisecond()
            ),
            (14, 5, 9, 123),
            "`A` is the time of day by itself"
        );
        assert_eq!(
            read(
                &compile("yyyy-MM-dd h b").unwrap(),
                "2024-03-01 3 PM",
                &zone
            )
            .unwrap()
            .hour(),
            15
        );
        assert_eq!(
            read(
                &compile("yyyy-MM-dd h B").unwrap(),
                "2024-03-01 3 at night",
                &zone
            )
            .unwrap()
            .hour(),
            3,
            "a period spanning both halves reads its midpoint, which is 01:30"
        );
        assert_eq!(
            read(
                &compile("yyyy-MM-dd h b").unwrap(),
                "2024-03-01 12 midnight",
                &zone
            )
            .unwrap()
            .hour(),
            0,
            "and one naming an instant fixes the hour outright"
        );
        assert_eq!(
            read(&compile("UUUU-MM-dd").unwrap(), "2024-03-01", &zone)
                .unwrap()
                .year(),
            2024
        );
        assert_eq!(
            read(&compile("rrrr-MM-dd").unwrap(), "-0043-03-15", &zone)
                .unwrap()
                .year(),
            -43,
            "`r` reads a sign back, which is the letter it resolves to"
        );

        // The three narrowing guards place each letter the way the field it
        // names is placed: the three time-of-day ones on a clock, `g` on a
        // date, and every zone spelling in neither and out of a parse pattern.
        for letter in ["A", "b", "B"] {
            let pieces = compile(letter).unwrap();
            assert!(date_fields_only(&pieces).is_err(), "`{letter}` on a date");
            assert!(time_fields_only(&pieces).is_ok(), "`{letter}` on a clock");
            assert!(civil_fields_only(&pieces).is_ok(), "`{letter}` in a parse");
        }
        for letter in ["g", "U", "r"] {
            let pieces = compile(letter).unwrap();
            assert!(date_fields_only(&pieces).is_ok(), "`{letter}` on a date");
            assert!(time_fields_only(&pieces).is_err(), "`{letter}` on a clock");
            assert!(civil_fields_only(&pieces).is_ok(), "`{letter}` in a parse");
        }
        for letter in ["z", "v", "O", "Z"] {
            let pieces = compile(letter).unwrap();
            assert!(date_fields_only(&pieces).is_err(), "`{letter}` on a date");
            assert!(time_fields_only(&pieces).is_err(), "`{letter}` on a clock");
            assert!(civil_fields_only(&pieces).is_err(), "`{letter}` in a parse");
        }
    }

    /// `Y` and `e` read the one week rule this module fixes, which is what
    /// makes them usable beside `w`: the year a week belongs to is the year
    /// that week's Thursday is in, and the weekday is counted from Monday.
    /// Asserted as agreement across the five letters that read the rule, so a
    /// letter that grew a week rule of its own fails here while still printing
    /// plausibly on its own line.
    #[test]
    fn a_week_based_year_and_local_weekday_read_the_territorys_week_data() {
        let zone = utc();
        let show = |pattern: &str, at: &Zoned| render(&compile(pattern).unwrap(), at);

        // 30 December 2019 is a Monday in ISO week 1 of 2020: the week-based
        // year runs ahead of the calendar one, which is the whole of why the
        // letter exists.
        let ahead = at(2019, 12, 30, 0, 0, 0, 0);
        assert_eq!(show("YYYY-'W'ww-e", &ahead), "2020-W01-1");
        assert_eq!(show("yyyy-MM-dd", &ahead), "2019-12-30");
        // And 1 January 2021 is a Friday still in 2020's week 53, which is the
        // same disagreement in the other direction.
        let behind = at(2021, 1, 1, 0, 0, 0, 0);
        assert_eq!(show("YYYY-'W'ww-e", &behind), "2020-W53-5");
        assert_eq!(show("yyyy-MM-dd", &behind), "2021-01-01");

        // `Y` takes `y`'s counts, including the two-digit window `u` does not
        // have, and `e` takes `c`'s — which is the agreement that says the two
        // letters read one rule rather than each their own.
        assert_eq!(show("Y YY YYYYY", &behind), "2020 20 02020");
        assert_eq!(
            show("e ee eee eeee eeeee eeeeee", &behind),
            "5 05 Fri Friday F Fr"
        );
        for day in 1..=7 {
            let value = at(2024, 1, day, 0, 0, 0, 0);
            assert_eq!(
                show("e", &value),
                show("c", &value),
                "`e` and `c` count the same week"
            );
            assert_eq!(
                show("Y", &value),
                show("YYYY", &value),
                "and `Y`'s counts pad one year"
            );
        }
        // The one week of the year where all five must agree at once: the
        // Monday that opens week 1 of 2024 falls in the December before it.
        assert_eq!(
            show("yyyy w W Y e c", &at(2024, 1, 1, 0, 0, 0, 0)),
            "2024 1 1 2024 1 1"
        );
        assert_eq!(
            show("yyyy w W Y e c", &at(2023, 12, 31, 0, 0, 0, 0)),
            "2023 52 4 2023 7 7",
            "the Sunday before it is the last day of 2023's week 52"
        );

        // `q` is `Q` under a second letter, which the root locale spells alike.
        assert_eq!(
            show("q qq qqq qqqq qqqqq", &behind),
            show("Q QQ QQQ QQQQ QQQQQ", &behind)
        );

        // Both are calendar fields, and both are read and discarded: a date is
        // built from `y`, `M` and `d`, so a week-based year beside them can
        // only agree or contradict.
        for letter in ["Y", "e", "q"] {
            let pieces = compile(letter).unwrap();
            assert!(date_fields_only(&pieces).is_ok(), "`{letter}` on a date");
            assert!(time_fields_only(&pieces).is_err(), "`{letter}` on a clock");
            assert!(civil_fields_only(&pieces).is_ok(), "`{letter}` in a parse");
        }
        let every = compile("YYYY-'W'ww-e yyyy-MM-dd").unwrap();
        let written = render(&every, &ahead);
        assert_eq!(written, "2020-W01-1 2019-12-30");
        let back = read(&every, &written, &zone).unwrap();
        assert_eq!((back.year(), back.month(), back.day()), (2019, 12, 30));

        // The letters CLDR reserves and gives no field to are still refused,
        // which is the other half of the same rule: `j` names a preference
        // between `h` and `H` in a skeleton, not a field in a pattern.
        for letter in ["j", "J", "C", "l", "i"] {
            assert!(compile(letter).is_err(), "`{letter}`");
        }
    }

    // ---------------------------------------------------- the plural rules

    use nvs_runtime::Decimal;

    /// The category a language puts a count in, through the member's own path
    /// from argument value to answer.
    fn category(count: Value, locale: &str) -> Category {
        let operands = operands_at(&[count], 0, PLURAL_MEMBER).unwrap();
        rules_for(subtag_of(locale, PLURAL_MEMBER).unwrap(), PLURAL_MEMBER)
            .unwrap()
            .select(operands)
    }

    /// [`category`]'s ordinal twin, through the second table.
    fn place(count: i64, locale: &str) -> Category {
        let operands = operands_at(&[Value::int(count)], 0, ORDINAL_MEMBER).unwrap();
        ordinal_rules_for(subtag_of(locale, ORDINAL_MEMBER).unwrap(), ORDINAL_MEMBER)
            .unwrap()
            .select(operands)
    }

    /// [`category`] over an `int`, which is what most of the assertions below
    /// hand it.
    fn whole(count: i64, locale: &str) -> Category {
        category(Value::int(count), locale)
    }

    /// A written count, which is the only kind that carries a scale — and so
    /// the only kind that can show `v` and `f` to a rule that reads them.
    fn dec(written: &str) -> Decimal {
        Decimal::parse(written).unwrap()
    }

    /// `rule:programs/framework-core-half`'s row, over the table this module carries: the six
    /// categories are all reachable, a language is answered from its own rules
    /// rather than from English's, the operands come from what the count
    /// shows, and a language the table does not carry is refused.
    // covers: Core\Cldr::pluralCategory
    #[test]
    fn plural_category_answers_from_the_carried_cldr_data() {
        use Category::{Few, Many, One, Other, Two, Zero};

        // Counted rather than read off a line: a table that answered `Other`
        // everywhere would still satisfy any single assertion about English.
        let reached: std::collections::BTreeSet<Category> = RULES
            .iter()
            .flat_map(|(_, rules)| {
                (0..=120u128).map(move |i| rules.select(Operands { i, v: 0, f: 0 }))
            })
            .collect();
        assert_eq!(
            reached.len(),
            6,
            "the carried rules reach only {reached:?}, so a `PluralCategory` case exists that \
             nothing can answer with"
        );

        // Russian reads the same five counts differently from English at four
        // of them, which is what a fallback to English would get wrong.
        let counts = [1, 2, 5, 11, 21];
        assert_eq!(
            counts.map(|n| whole(n, "ru")),
            [One, Few, Many, Many, One],
            "Russian's `one` is 1 and 21 but not 11, and its `few` is 2 to 4"
        );
        assert_eq!(
            counts.map(|n| whole(n, "en")),
            [One, Other, Other, Other, Other],
            "English draws the line at 1 and nowhere else"
        );

        // Welsh is the language that uses all six, and it is why the enum has
        // six cases rather than the two English needs.
        assert_eq!(
            [0, 1, 2, 3, 6, 4].map(|n| whole(n, "cy")),
            [Zero, One, Two, Few, Many, Other]
        );

        // Arabic's `few` and `many` are moduli rather than counts, so they
        // catch 103 and 111 as well as 3 and 11.
        assert_eq!([3, 103].map(|n| whole(n, "ar")), [Few, Few]);
        assert_eq!([11, 111].map(|n| whole(n, "ar")), [Many, Many]);

        // The visible fraction is an operand. English's `one` is
        // `i = 1 and v = 0`, so a `decimal` carrying a scale is not it, while
        // the `int` and the `float` both show `1` and are.
        assert_eq!(category(Value::int(1), "en"), One);
        assert_eq!(category(Value::float(1.0), "en"), One);
        assert_eq!(
            category(Value::decimal(Decimal::parse("1.0").unwrap()), "en"),
            Other,
            "`1.0` shows a fraction digit, and English's singular does not"
        );
        // Czech reads the same fact the other way: any fraction at all is
        // `many` there, whatever the integer part is.
        assert_eq!(
            category(Value::decimal(Decimal::parse("1.5").unwrap()), "cs"),
            Many
        );

        // Only the language subtag is read, and its case is not significant.
        assert_eq!(whole(2, "en-GB"), Other);
        assert_eq!(whole(2, "en_US"), Other);
        assert_eq!(whole(2, "RU"), Few);
        assert_eq!(whole(2, "pt-BR"), Other);

        // A count that is not finite has no category, and neither does one
        // whose digits outrun the operands.
        assert!(operands_at(&[Value::float(f64::NAN)], 0, PLURAL_MEMBER).is_err());
        assert!(operands_at(&[Value::float(1e300)], 0, PLURAL_MEMBER).is_err());
    }

    /// The module doc's roster section says `RULES` names no absence, and this
    /// is what holds that claim to the table: every language below answers, and
    /// answers with its *own* published rule rather than with a neighbour's.
    ///
    /// Asserted as a disagreement rather than as a category per language: each
    /// count below is one CLDR puts in a different place for the named
    /// language than for the set it would most plausibly have been filed
    /// under, so a row pointing at the wrong arm fails here while still
    /// answering plausibly on its own line.
    #[test]
    fn every_language_named_absent_in_the_gap_note_now_has_a_rule() {
        use Category::{Few, Many, One, Other, Two, Zero};

        let named = [
            "be", "he", "mt", "dsb", "hsb", "gd", "br", "kw", "gv", "is", "mk", "tzm", "shi", "si",
            "ak", "bh", "guw", "nso", "wa", "naq",
        ];
        for subtag in named {
            assert!(
                rules_for(subtag, PLURAL_MEMBER).is_ok(),
                "`{subtag}` is still absent, and the module doc's roster section says it is not"
            );
        }

        // Belarusian reads `n` where Russian reads `i`, so the two agree on
        // every whole count and part company at the first fraction: 1.5 is
        // outside every Belarusian band and inside Russian's `Other` too —
        // the disagreement is at 5.5, which `many`'s `n % 10 = 5` takes.
        assert_eq!(
            [1, 2, 5, 11, 21].map(|n| whole(n, "be")),
            [One, Few, Many, Many, One]
        );
        assert_eq!(category(Value::decimal(dec("5.5")), "be"), Other);
        assert_eq!(category(Value::decimal(dec("5.5")), "ru"), Other);

        // Hebrew: `one` also takes a count with no integer part, which is the
        // clause no other carried set has.
        assert_eq!([1, 2, 3].map(|n| whole(n, "he")), [One, Two, Other]);
        assert_eq!(category(Value::decimal(dec("0.5")), "he"), One);
        assert_eq!(category(Value::decimal(dec("1.0")), "he"), Other);

        // Maltese: zero is `few` rather than a `zero`, and the teens are
        // `many` at 11 and at 111 alike.
        assert_eq!(
            [0, 1, 2, 3, 111, 200].map(|n| whole(n, "mt")),
            [Few, One, Two, Few, Many, Other]
        );

        // Sorbian is Slovenian's `i % 100` bands with the fraction digits read
        // the same way, so `0.1` is `one` there and `few` in Slovenian.
        assert_eq!(
            [1, 2, 3, 5, 101].map(|n| whole(n, "dsb")),
            [One, Two, Few, Other, One]
        );
        assert_eq!(whole(2, "hsb"), Two);
        assert_eq!(category(Value::decimal(dec("0.1")), "dsb"), One);
        assert_eq!(category(Value::decimal(dec("0.1")), "sl"), Few);

        // Scottish Gaelic repeats its first three bands once in the teens,
        // which Irish — the set it sits nearest — does not.
        assert_eq!(
            [1, 11, 2, 12, 13, 19, 20].map(|n| whole(n, "gd")),
            [One, One, Two, Two, Few, Few, Other]
        );
        assert_eq!([11, 12].map(|n| whole(n, "ga")), [Other, Other]);

        // Breton excludes 71 and 91 from `one` and keeps a millions `many`.
        assert_eq!(
            [1, 71, 2, 72, 3, 9, 10].map(|n| whole(n, "br")),
            [One, Other, Two, Other, Few, Few, Other]
        );
        assert_eq!(whole(2_000_000, "br"), Many);

        // Cornish reaches all six, and its `two` is a modulus rather than the
        // count — 22 and 42 are `two` where 21 and 41 are `many`.
        assert_eq!(
            [0, 1, 22, 42, 3, 23, 21, 41, 5].map(|n| whole(n, "kw")),
            [Zero, One, Two, Two, Few, Few, Many, Many, Other]
        );

        // Manx: `many` is any visible fraction at all, and `few` is the even
        // twenties rather than a band.
        assert_eq!(
            [1, 2, 20, 40, 3].map(|n| whole(n, "gv")),
            [One, Two, Few, Few, Other]
        );
        assert_eq!(category(Value::decimal(dec("1.5")), "gv"), Many);

        // Icelandic is the opposite reading of a fraction: 1.5 joins `one`
        // rather than leaving it, which is what `t != 0` says.
        assert_eq!(
            [1, 11, 21, 2].map(|n| whole(n, "is")),
            [One, Other, One, Other]
        );
        assert_eq!(category(Value::decimal(dec("1.5")), "is"), One);
        assert_eq!(category(Value::decimal(dec("2.5")), "is"), One);

        // Macedonian is Serbo-Croatian's `one` with nothing behind it, so 2 to
        // 4 are `other` here and `few` there.
        assert_eq!(
            [1, 21, 11, 3].map(|n| whole(n, "mk")),
            [One, One, Other, Other]
        );
        assert_eq!(whole(3, "sr"), Few);

        // Tamazight's `one` is two disjoint bands, so it reopens at 11.
        assert_eq!(
            [0, 1, 2, 11, 99, 100].map(|n| whole(n, "tzm")),
            [One, One, Other, One, One, Other]
        );

        // Tachelhit has a `few` behind an `i = 0 or n = 1` singular.
        assert_eq!(
            [0, 1, 2, 10, 11].map(|n| whole(n, "shi")),
            [One, One, Few, Few, Other]
        );
        assert_eq!(category(Value::decimal(dec("0.5")), "shi"), One);

        // Sinhala's `one` takes exactly one fraction, `0.1`.
        assert_eq!([0, 1, 2].map(|n| whole(n, "si")), [One, One, Other]);
        assert_eq!(category(Value::decimal(dec("0.1")), "si"), One);
        assert_eq!(category(Value::decimal(dec("0.2")), "si"), Other);

        // The five on `n = 0..1` agree with each other and disagree with
        // `i = 0,1` at 1.5, which is the whole reason they are a second arm.
        for subtag in ["ak", "bh", "guw", "nso", "wa"] {
            assert_eq!([0, 1, 2].map(|n| whole(n, subtag)), [One, One, Other]);
            assert_eq!(category(Value::decimal(dec("1.5")), subtag), Other);
        }
        assert_eq!(category(Value::decimal(dec("1.5")), "hy"), One);

        // Nama distinguishes one and two and nothing else.
        assert_eq!([1, 2, 3].map(|n| whole(n, "naq")), [One, Two, Other]);

        // The four that went in beside the twenty, for the same reason a
        // roster with a hole in it is worse than a small one.
        assert_eq!([1, 2].map(|n| whole(n, "da")), [One, Other]);
        assert_eq!(category(Value::decimal(dec("0.5")), "da"), One);
        assert_eq!(category(Value::decimal(dec("2.5")), "da"), Other);
        for subtag in ["fil", "tl", "ceb"] {
            assert_eq!(
                [1, 2, 3, 5, 4, 6, 9].map(|n| whole(n, subtag)),
                [One, One, One, One, Other, Other, Other]
            );
        }
    }

    /// The refusal survives the roster's widening: a language CLDR has no
    /// published rule for is still refused rather than given English's, and
    /// the message still names it.
    ///
    /// Its own test rather than a line in the sweep above, because the roster
    /// growing is exactly the change that would quietly turn the refusal into
    /// a fallback.
    #[test]
    fn a_language_with_no_published_rule_still_throws_naming_itself() {
        let refused =
            rules_for(subtag_of("tlh-Piqd", PLURAL_MEMBER).unwrap(), PLURAL_MEMBER).unwrap_err();
        assert!(
            format!("{refused:?}").contains("tlh"),
            "the refusal has to name the language it could not answer for"
        );
        // A subtag that is a prefix of a carried row is not that row: `e` is
        // not `en`, and a bisection that answered it would be a fallback with
        // no one to notice it.
        assert!(rules_for("e", PLURAL_MEMBER).is_err());
        assert!(rules_for("zzz", PLURAL_MEMBER).is_err());
        assert!(subtag_of("", PLURAL_MEMBER).is_err());
        assert!(subtag_of("-GB", PLURAL_MEMBER).is_err());
        // The ordinal member draws the same boundary, and names itself doing
        // it — the fallback below is for a language the *cardinal* table
        // carries, never for one it does not.
        let ordinal = ordinal_rules_for("tlh", ORDINAL_MEMBER).unwrap_err();
        assert!(format!("{ordinal:?}").contains("ordinalCategory"));
    }

    /// The second table, swept: every language [`RULES`] carries has an
    /// ordinal answer, every row of [`ORDINALS`] answers its own published
    /// rule, and the default the module doc argues for is the one a language
    /// off that roster gets.
    // covers: Core\Cldr::ordinalCategory
    #[test]
    fn an_ordinal_category_is_answered_for_every_language_with_a_published_table() {
        use Category::{Few, Many, One, Other, Two, Zero};

        // Every ordinal row is a cardinal row too, so the fallback below is
        // the only way to reach `Unmarked` and no row is unreachable through
        // the member's own lookup.
        for (subtag, rules) in ORDINALS {
            assert_eq!(
                ordinal_rules_for(&subtag.to_ascii_uppercase(), ORDINAL_MEMBER).unwrap(),
                *rules
            );
            assert!(
                rules_for(subtag, PLURAL_MEMBER).is_ok(),
                "`{subtag}` marks an ordinal form and is not in the cardinal roster"
            );
        }
        // Counted rather than read off a line: the six categories are all
        // reachable through this table, which is what registering no second
        // enum for it rests on.
        let reached: std::collections::BTreeSet<Category> = ORDINALS
            .iter()
            .flat_map(|(_, rules)| {
                (0..=900u128).map(move |i| rules.select(Operands { i, v: 0, f: 0 }))
            })
            .collect();
        assert_eq!(
            reached.len(),
            6,
            "the ordinal table reaches only {reached:?}"
        );

        // And every language the cardinal roster carries is answered, which is
        // the claim the default exists to make: a message that writes `1st`
        // never has to ask whether its locale is on a second list.
        for (subtag, _) in RULES {
            assert!(ordinal_rules_for(subtag, ORDINAL_MEMBER).is_ok());
        }

        assert_eq!(
            [1, 2, 3, 4, 11, 12, 13, 21].map(|n| place(n, "en")),
            [One, Two, Few, Other, Other, Other, Other, One]
        );
        assert_eq!(
            [1, 2, 11, 12, 21, 22].map(|n| place(n, "sv")),
            [One, One, Other, Other, One, One]
        );
        assert_eq!([3, 13, 23].map(|n| place(n, "uk")), [Few, Other, Few]);
        assert_eq!(
            [2, 3, 12, 13, 22].map(|n| place(n, "be")),
            [Few, Few, Other, Other, Few]
        );
        assert_eq!(
            [1, 8, 11, 80, 800, 8_000].map(|n| place(n, "it")),
            [Other, Many, Many, Many, Many, Other]
        );
        assert_eq!(place(8, "scn"), Many);
        assert_eq!([1, 2].map(|n| place(n, "fr")), [One, Other]);
        assert_eq!([1, 5, 2].map(|n| place(n, "hu")), [One, One, Other]);
        assert_eq!([1, 4, 5].map(|n| place(n, "ne")), [One, One, Other]);
        assert_eq!(
            [1, 3, 2, 4, 5].map(|n| place(n, "ca")),
            [One, One, Two, Few, Other]
        );
        assert_eq!(
            [1, 2, 3, 4, 6].map(|n| place(n, "mr")),
            [One, Two, Two, Few, Other]
        );
        assert_eq!(
            [1, 2, 3, 4, 6, 5].map(|n| place(n, "hi")),
            [One, Two, Two, Few, Many, Other]
        );
        assert_eq!(
            [1, 5, 10, 2, 4, 6, 11].map(|n| place(n, "bn")),
            [One, One, One, Two, Few, Many, Other]
        );
        assert_eq!([9, 10].map(|n| place(n, "or")), [One, Other]);
        assert_eq!(
            [1, 20, 3, 100, 0, 6, 40, 9].map(|n| place(n, "az")),
            [One, One, Few, Few, Many, Many, Many, Other]
        );
        assert_eq!(
            [1, 4, 14, 24].map(|n| place(n, "sq")),
            [One, Many, Other, Many]
        );
        assert_eq!(
            [6, 9, 10, 0, 1].map(|n| place(n, "kk")),
            [Many, Many, Many, Other, Other]
        );
        assert_eq!(
            [0, 1, 2, 3, 5, 10].map(|n| place(n, "cy")),
            [Zero, One, Two, Few, Many, Other]
        );
        assert_eq!(
            [1, 11, 2, 7, 17].map(|n| place(n, "mk")),
            [One, Other, Two, Many, Other]
        );
        assert_eq!(
            [1, 0, 2, 20, 40, 21].map(|n| place(n, "ka")),
            [One, Many, Many, Many, Many, Other]
        );
        assert_eq!(
            [6, 9, 10, 1].map(|n| place(n, "tk")),
            [Few, Few, Few, Other]
        );
        // Cornish on both sides of a band it reopens: the fourth is `one`, the
        // fifth is `many`, the sixth is neither, and a hundred reads the band
        // again rather than the count.
        assert_eq!(
            [4, 5, 6, 21, 24, 25, 100, 101, 105].map(|n| place(n, "kw")),
            [One, Many, Other, One, One, Other, Other, One, Many]
        );
        for subtag in ["ga", "lo", "mo", "ms", "ro", "vi"] {
            assert_eq!([1, 2].map(|n| place(n, subtag)), [One, Other]);
        }

        // The default, and the disagreement that is the whole reason this is a
        // second table: German and Japanese mark no ordinal form, and English
        // and Russian read 2 the opposite way round from each other.
        for subtag in ["de", "ja", "ru", "pl"] {
            assert_eq!(
                [1, 2, 3, 4].map(|n| place(n, subtag)),
                [Other, Other, Other, Other]
            );
        }
        assert_eq!((place(2, "en"), whole(2, "en")), (Two, Other));
        assert_eq!((place(2, "ru"), whole(2, "ru")), (Other, Few));

        // A fraction is outside every rule stated over `n`, and inside the two
        // stated over `i`.
        assert_eq!(
            ordinal_rules_for("en", ORDINAL_MEMBER)
                .unwrap()
                .select(Operands { i: 1, v: 1, f: 5 }),
            Other
        );
        assert_eq!(
            ordinal_rules_for("ka", ORDINAL_MEMBER)
                .unwrap()
                .select(Operands { i: 1, v: 1, f: 5 }),
            One
        );
    }

    /// The direction the sweep above does not run: every language CLDR
    /// publishes a marked ordinal rule for is on [`ORDINALS`], the roster
    /// carries nothing CLDR publishes no marked rule for, and no two published
    /// rules are one rule under two names.
    ///
    /// The groups below are CLDR's own ordinal locale lists, over the cardinal
    /// roster this table is closed against — the module doc's ordinal section
    /// owns that closure, and a language CLDR marks that [`RULES`] does not
    /// carry is a row in *that* table first, refused by both members until it
    /// is one.
    #[test]
    fn every_language_cldr_gives_an_ordinal_rule_is_on_the_ordinal_roster() {
        let published: &[(OrdinalSet, &[&str])] = &[
            (OrdinalSet::Swedish, &["sv"]),
            (OrdinalSet::English, &["en"]),
            (
                OrdinalSet::FirstOnly,
                &["fil", "fr", "ga", "hy", "lo", "mo", "ms", "ro", "tl", "vi"],
            ),
            (OrdinalSet::Hungarian, &["hu"]),
            (OrdinalSet::Nepali, &["ne"]),
            (OrdinalSet::Belarusian, &["be"]),
            (OrdinalSet::Ukrainian, &["uk"]),
            (OrdinalSet::Turkmen, &["tk"]),
            (OrdinalSet::Kazakh, &["kk"]),
            (OrdinalSet::Italian, &["it", "sc", "scn"]),
            (OrdinalSet::Georgian, &["ka"]),
            (OrdinalSet::Albanian, &["sq"]),
            (OrdinalSet::Cornish, &["kw"]),
            (OrdinalSet::Macedonian, &["mk"]),
            (OrdinalSet::Azerbaijani, &["az"]),
            (OrdinalSet::Catalan, &["ca"]),
            (OrdinalSet::Marathi, &["mr"]),
            (OrdinalSet::Hindi, &["gu", "hi"]),
            (OrdinalSet::Bengali, &["as", "bn"]),
            (OrdinalSet::Odia, &["or"]),
            (OrdinalSet::Welsh, &["cy"]),
        ];

        for (rules, locales) in published {
            for subtag in *locales {
                assert_eq!(
                    ordinal_rules_for(subtag, ORDINAL_MEMBER).unwrap(),
                    *rules,
                    "`{subtag}` is published as {rules:?} and the roster answers otherwise"
                );
            }
        }

        // Counted rather than read off a line: every assertion above passes
        // unchanged with a row here that CLDR marks nothing for, and one of
        // those is a form invented for a language that writes none.
        let listed: usize = published.iter().map(|(_, locales)| locales.len()).sum();
        assert_eq!(
            ORDINALS.len(),
            listed,
            "the roster and CLDR's published ordinal data are different lengths"
        );

        // And no arm is another arm's rule under a second name, which is what
        // makes each one a transcription rather than a language's label: the
        // pair that renders identically at every count is one rule, and the
        // one that renders as the default marks nothing.
        let answers = |rules: OrdinalSet| {
            (0..=1_000u128)
                .map(|count| {
                    rules.select(Operands {
                        i: count,
                        v: 0,
                        f: 0,
                    })
                })
                .collect::<Vec<_>>()
        };
        let unmarked = answers(OrdinalSet::Unmarked);
        for (index, (rules, locales)) in published.iter().enumerate() {
            assert_ne!(
                answers(*rules),
                unmarked,
                "{rules:?} marks nothing, so {locales:?} belongs off the roster"
            );
            for (other, named) in &published[index + 1..] {
                assert_ne!(
                    answers(*rules),
                    answers(*other),
                    "{rules:?} and {other:?} answer alike, so {locales:?} and {named:?} are one \
                     group"
                );
            }
        }
    }

    /// [`RULES`] is bisected and folded against, so both properties it is read
    /// under are asserted rather than assumed — a row out of order makes
    /// [`rules_for`] miss a language it carries, which no assertion about a
    /// language already tested would catch.
    #[test]
    fn the_locale_table_is_sorted_and_lower_case() {
        assert!(
            RULES.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "{:?} is out of order",
            RULES
                .windows(2)
                .find(|pair| pair[0].0 >= pair[1].0)
                .map(|pair| (pair[0].0, pair[1].0))
        );
        assert!(
            RULES
                .iter()
                .all(|(name, _)| name.bytes().all(|byte| byte.is_ascii_lowercase())),
            "a row that is not lower case can never be found, since the needle is folded down"
        );
        // Every row is reachable through the member's own lookup, folded.
        for (name, rules) in RULES {
            assert_eq!(
                rules_for(&name.to_ascii_uppercase(), PLURAL_MEMBER).unwrap(),
                *rules
            );
        }
        // The ordinal table is bisected and folded the same way, so it owes
        // the same two properties.
        assert!(
            ORDINALS.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "{:?} is out of order",
            ORDINALS
                .windows(2)
                .find(|pair| pair[0].0 >= pair[1].0)
                .map(|pair| (pair[0].0, pair[1].0))
        );
        assert!(
            ORDINALS
                .iter()
                .all(|(name, _)| name.bytes().all(|byte| byte.is_ascii_lowercase()))
        );
    }

    /// The ordinal a helper hands back is the case a program compares against,
    /// and the two are written in different places.
    #[test]
    fn an_ordinal_is_its_case_in_the_registered_enum() {
        for (category, case) in [
            (Category::Zero, "Zero"),
            (Category::One, "One"),
            (Category::Two, "Two"),
            (Category::Few, "Few"),
            (Category::Many, "Many"),
            (Category::Other, "Other"),
        ] {
            let index = usize::try_from(category.ordinal()).unwrap();
            assert_eq!(PLURAL_CATEGORY.cases[index].0, case);
            assert_eq!(PLURAL_CATEGORY.cases[index].1, category.ordinal());
        }
        assert_eq!(PLURAL_CATEGORY.cases.len(), 6);
    }
}
