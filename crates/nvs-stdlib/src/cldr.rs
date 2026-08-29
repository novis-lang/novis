//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 4's date pattern grammar — the one `Core\Time\DateTime::format` emits
//! from and `Core\Time::parse` reads with, in **CLDR** letters
//! (`yyyy-MM-dd HH:mm:ss`, `EEEE, d MMMM yyyy`) rather than PHP's `date()`
//! ones.
//!
//! # Why this is written here rather than taken from a crate
//!
//! [`jiff`] already carries a pattern engine, but it is `strftime`'s — a
//! *third* letter grammar, neither the one § 4 wrote nor the one PHP wrote. A
//! full CLDR implementation is `icu`, which is a locale-data dependency an
//! order of magnitude larger than everything else in `Core` put together and
//! which ADR 0051 § 4's second question ("does it carry weight the language
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
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) shuts on
//! `setlocale`: a process-wide setting that silently changes what a later
//! `format` answers is exactly the ambient state § 4 removed the default
//! timezone for. A program that wants a localized month name has the number,
//! and localization is an application concern rather than a Tier 0 one.
//!
//! # The subset
//!
//! | Letter | Meaning | Counts |
//! |---|---|---|
//! | `y` | year | `yy` is two digits, any other count zero-pads |
//! | `M` | month | 1–2 numeric, `MMM` short name, `MMMM` full, `MMMMM` narrow |
//! | `d` | day of month | numeric, zero-padded to the count |
//! | `D` | day of year | numeric |
//! | `E` | weekday name | 1–3 short, `EEEE` full, `EEEEE` narrow, `EEEEEE` two-letter |
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
//! # Known gaps
//!
//! 1. **A pattern is compiled per call.** [ADR 0057](../../../../docs/adr/0057-intrinsic-literal-folding.md)
//!    makes `format`/`parse` intrinsics whose *literal* pattern is validated
//!    and prepared while compiling, which is the same work [`compile`] does
//!    and would move it off the request path; the reported diagnostic would
//!    then be a compile error rather than the throw [`compile`] returns
//!    today. Nothing about this module changes when that lands — it gains a
//!    second caller.
//! 2. **Era, quarter, week-of-year and the standalone forms (`G`, `Q`, `w`,
//!    `W`, `L`, `c`, `F`, `u`) are refused**, each naming itself. They are
//!    additions to the table above rather than a different design; `Q` is the
//!    only one § 4 names elsewhere, as a `Unit` case rather than a pattern
//!    letter.

use jiff::Zoned;
use jiff::civil;
use jiff::tz::TimeZone;

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
    fn is_calendar(self) -> bool {
        matches!(
            self,
            Self::Year | Self::Month | Self::Day | Self::DayOfYear | Self::Weekday
        )
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
/// [ADR 0057](../../../../docs/adr/0057-intrinsic-literal-folding.md)'s fold,
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
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Literal(text) => out.push_str(text),
            Piece::Field(field, count) => render_field(&mut out, *field, *count, at),
        }
    }
    out
}

/// One field of [`render`].
#[expect(
    clippy::cast_sign_loss,
    reason = "every component read here is non-negative by construction but \
              typed `i8`/`i16` by `jiff`; the year is the one that can be \
              negative, and it takes its absolute value first"
)]
fn render_field(out: &mut String, field: Field, count: usize, at: &Zoned) {
    match field {
        Field::Year => {
            let year = at.year();
            if year < 0 {
                out.push('-');
            }
            let magnitude = u32::from(year.unsigned_abs());
            if count == 2 {
                pad(out, u64::from(magnitude % 100), 2);
            } else {
                pad(out, u64::from(magnitude), count);
            }
        }
        Field::Month => match count {
            3 => out.push_str(MONTHS[at.month() as usize - 1].1),
            4 => out.push_str(MONTHS[at.month() as usize - 1].0),
            5 => out.push_str(MONTHS[at.month() as usize - 1].2),
            _ => pad(out, at.month() as u64, count),
        },
        Field::Day => pad(out, at.day() as u64, count),
        Field::DayOfYear => pad(out, at.day_of_year() as u64, count),
        Field::Weekday => {
            let names = WEEKDAYS[at.weekday().to_monday_zero_offset() as usize];
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
            let seconds = at.offset().seconds();
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
        Field::ZoneId => out.push_str(at.time_zone().iana_name().unwrap_or("UTC")),
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
        Field::Month if count >= 3 => {
            let index = name_index(bytes, at, "month", |index| {
                let names = MONTHS[index];
                [names.0, names.1, names.2]
            })?;
            fields.month =
                Some(i8::try_from(index).expect("a month index is between one and twelve"));
        }
        Field::Month => fields.month = Some(small(bytes, at, count, "month")?),
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
            name_index(bytes, at, "weekday", |index| {
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

/// Matches one of the names `names` gives for each index, longest first so
/// that `"June"` is not read as the short `"Jun"` with a stray `e` after it.
fn name_index(
    bytes: &[u8],
    at: &mut usize,
    what: &str,
    names: impl Fn(usize) -> [&'static str; 3],
) -> Result<usize, String> {
    let mut best: Option<(usize, usize)> = None;
    for index in 0..if what == "month" { 12 } else { 7 } {
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
    Ok(index + usize::from(what == "month"))
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
        assert!(compile("yyyy Q").unwrap_err().contains("`Q` is not"));
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
}
