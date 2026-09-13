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
//! **The week rule is ISO 8601's, for `w` and `W` alike**: a week starts on
//! Monday, and week 1 is the first one with at least four days in the period.
//! CLDR states that rule per *territory* rather than per language, which is
//! exactly the locale data the section above refuses to carry — so one rule is
//! written here, it is the one the majority of that table names, and a `W`
//! whose month opens with a short partial week answers `0`, as ICU's own
//! week-of-month does.
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
//! # Known gaps
//!
//! 1. **A pattern is compiled per call.** `rule:expressions/intrinsic-literals`
//!    makes `format`/`parse` intrinsics whose *literal* pattern is validated
//!    and prepared while compiling, which is the same work [`compile`] does
//!    and would move it off the request path; the reported diagnostic would
//!    then be a compile error rather than the throw [`compile`] returns
//!    today. Nothing about this module changes when that lands — it gains a
//!    second caller.
//!    Decided: Build the checker-to-IR channel and prepare literal patterns at compile time — Zero
//!    request-path parsing and compile-time errors as the rule says; the channel is new plumbing
//!    (shared with nvs-types' intrinsic gaps).
//!    — owner: unowned-closures
//! 2. **The letters still refused are the ones needing data or a second
//!    calendar** — `Y` and `e` (week-based year and local weekday number, both
//!    of which read the per-territory week data this module does not carry),
//!    `U` and `r` (a cyclic calendar's year), `B` and `b` (flexible day
//!    periods, which are locale data), `A` (milliseconds in the day), `g`
//!    (modified Julian day), and the four zone spellings `z`, `Z`, `O` and
//!    `v`, which name a zone the way `X`, `x` and `VV` already do. Each names
//!    itself rather than emitting a literal. `Y` is the one with a caller
//!    waiting, since a week-based year beside `w` is the pair ISO 8601 writes.
//!    — owner: gap-zero
//! 3. **[`RULES`] is a roster, not all of CLDR**, and it names no absence.
//!    It carries the languages whose published cardinal rules are transcribed
//!    here; every other one throws, per the section above. The twenty this
//!    note used to name — `be`, `he`, `mt`, `dsb`, `hsb`, `gd`, `br`, `kw`,
//!    `gv`, `is`, `mk`, `tzm`, `shi`, `si`, `ak`, `bh`, `guw`, `nso`, `wa`,
//!    `naq` — are carried, as fifteen arms between them, and `da`, `fil`,
//!    `tl` and `ceb` went in beside them because a roster that answers for
//!    `af` and refuses Danish is a roster with a hole rather than a boundary.
//!    `every_language_named_absent_in_the_gap_note_now_has_a_rule` is what
//!    holds this paragraph to the table.
//!    — owner: gap-zero
//! 4. **[`ORDINALS`] is the languages that mark a form, and a language that
//!    marks one but is missing from it answers `Other` silently** — which is
//!    the cost of the default the section above argues for, stated plainly.
//!    The cardinal roster has no such failure mode: a missing row there
//!    throws. Widening this one is a row, and an arm only where the published
//!    rule is a shape no arm has.
//!    — owner: gap-zero

use std::cmp::Ordering;
use std::ops::RangeInclusive;

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
}

impl Field {
    /// The field a CLDR pattern letter names, or `None` for one this subset
    /// does not carry — see this module's gap 2.
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
            _ => return None,
        })
    }

    /// Whether this field says something about the **zone** rather than about
    /// a civil field — which is what makes it legal in a `format` pattern and
    /// refused in a `parse` one, since `Core\Time::parse` takes the zone as
    /// its own third argument.
    fn is_zonal(self) -> bool {
        matches!(self, Self::OffsetZ | Self::Offset | Self::ZoneId)
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
        if field == Field::ZoneId && count != 2 {
            return Err("`V` names a zone only as `VV`, the IANA identifier".to_owned());
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
/// The *member* restrictions are deliberately not applied: `Core\Time::parse`
/// also refuses a zonal field (`civil_fields_only`), which is a rule about
/// that member rather than about the grammar, and leaving it to run time keeps
/// this answer sound — everything it refuses, every caller refuses.
pub fn validate(pattern: &str) -> Result<(), String> {
    compile(pattern).map(|_| ())
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
              typed `i8`/`i16` by `jiff`; the extended year is the one that \
              can be negative, and it takes its absolute value first"
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
        Field::Year | Field::ExtendedYear => {
            // One arm for both, because this subset's `y` is already the signed
            // proleptic year the module doc argues for — `yy`'s two-digit
            // window is the only thing that separates them here.
            let year = at.year();
            if year < 0 {
                out.push('-');
            }
            let magnitude = u32::from(year.unsigned_abs());
            if count == 2 && field == Field::Year {
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
        Field::StandaloneWeekday => {
            let index = at.date().weekday().to_monday_zero_offset() as usize;
            let names = WEEKDAYS[index];
            match count {
                3 => out.push_str(names.1),
                4 => out.push_str(names.0),
                5 => out.push_str(names.2),
                6 => out.push_str(names.3),
                // CLDR's `c` counts the local day of the week, which is
                // Monday-first in the root locale — the same order [`WEEKDAYS`]
                // is indexed in, so it is the index plus one.
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
        Field::StandaloneWeekday if count >= 3 => {
            name_index(bytes, at, "weekday", WEEKDAYS.len(), |index| {
                let names = WEEKDAYS[index];
                [names.0, names.1, names.3]
            })?;
        }
        Field::StandaloneWeekday => {
            small(bytes, at, count, "weekday")?;
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
        Field::OffsetZ | Field::Offset | Field::ZoneId => {
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
    /// French, Armenian, Filipino, Tagalog: the first and nothing else.
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
/// from `Other`, which is how CLDR's own ordinal data is written. The module
/// doc's ordinal section is the home of why the absence answers rather than
/// throws, and `an_ordinal_category_is_answered_for_every_language_with_a_published_table`
/// is what holds every row here to a row there.
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
    ("gu", OrdinalSet::Hindi),
    ("hi", OrdinalSet::Hindi),
    ("hu", OrdinalSet::Hungarian),
    ("hy", OrdinalSet::FirstOnly),
    ("it", OrdinalSet::Italian),
    ("ka", OrdinalSet::Georgian),
    ("kk", OrdinalSet::Kazakh),
    ("mk", OrdinalSet::Macedonian),
    ("mr", OrdinalSet::Marathi),
    ("ne", OrdinalSet::Nepali),
    ("or", OrdinalSet::Odia),
    ("sc", OrdinalSet::Italian),
    ("scn", OrdinalSet::Italian),
    ("sq", OrdinalSet::Albanian),
    ("sv", OrdinalSet::Swedish),
    ("tk", OrdinalSet::Turkmen),
    ("tl", OrdinalSet::FirstOnly),
    ("uk", OrdinalSet::Ukrainian),
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
        assert!(compile("yyyy Y").unwrap_err().contains("`Y` is not"));
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

    /// The module doc's gap 3 used to name twenty languages the roster did not
    /// carry. It names none now, and this is what holds it to that: every one
    /// of the twenty answers, and answers with its *own* published rule rather
    /// than with a neighbour's.
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
                "`{subtag}` is still absent, and the module doc's gap 3 says it is not"
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

    /// Gap 4's second table: every language [`RULES`] carries has an ordinal
    /// answer, every row of [`ORDINALS`] answers its own published rule, and
    /// the default the module doc argues for is the one a language off that
    /// roster gets.
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
