//! The ISO 8601 texts `Core\Time`'s `fromIso` members read, at the bottom of the
//! crate tree: one reader per class, `Instant`, `Date`, `TimeOfDay` and
//! `DateTime`, each returning the `jiff` value it names.
//!
//! # Why the grammar is here and not beside the classes
//!
//! The classes are `nvs_stdlib::time`'s: their slots, their members and their
//! refusals. What is *not* theirs is the question [`crate::routes`] asks.
//! `rule:security/route-capture-is-laundered-by-its-type` lets a capture and a
//! query value declare `Date`, `TimeOfDay`, `Instant` and, for a query value,
//! `DateTime`, and a capture is accepted or refused in this crate, one *below*
//! the one that owns the types. So the readers sit down here, for the reason
//! [`crate::uuid`] and [`crate::decimal`] do: a matcher that cannot perform a
//! conversion matches a segment the declared type would have refused.
//!
//! **One reader per class, not two.** Each `fromIso` member, the checker's test
//! of a written literal, the matcher and a command's argument all read through
//! the functions below, so a text one of them accepts is a text each of them
//! accepts.
//!
//! **What it spends:** a fixed grammar over at most about 50 bytes, and no
//! allocation but the error sentence. A `DateTime` text looks its zone up in
//! the bundled database once, and a name longer than [`ZONE_NAME_MAX`] is
//! refused before the lookup, so a long bracket costs one length check.

use jiff::civil;
use jiff::tz::{Offset, TimeZone};
use jiff::{Timestamp, Zoned};

/// Why a reader refuses a time past either end of the timestamp range. It
/// replaces jiff's own sentence, which names a parameter the program never
/// wrote.
pub const PAST_THE_RANGE: &str = "the time is outside -9999-01-02T01:59:59Z..=9999-12-30T22:00:00Z";

/// The longest zone name [`written_zone`] looks up. Every IANA name in the
/// database is shorter, which `every_zone_name_fits_the_bound` checks, so a
/// longer name is refused without a lookup.
pub const ZONE_NAME_MAX: usize = 64;

/// The instant `text` names: `Instant::fromIso`'s grammar.
///
/// A date, `T` or one space, a time as [`time_of_day`] reads it, and then `Z`
/// or a `±HH:MM` offset. The date and the time are read by the readers
/// [`local`] uses, so a leap second `60`, a lower-case `t`, a comma before the
/// fraction and a basic offset such as `+0100` are refused here as they are
/// there. A text that is written as one and still fails names a time past
/// either end of the range.
///
/// # Errors
///
/// The sentence `Instant::fromIso` throws with.
pub fn instant(text: &str) -> Result<Timestamp, String> {
    const FORM: &str = "YYYY-MM-DDTHH:MM:SSZ";
    let bytes = text.as_bytes();
    let (date, end) = date_prefix(bytes)?;
    if !matches!(bytes.get(end), Some(b'T' | b' ')) {
        return Err(format!(
            "expected `T` or a space at byte {end}, between the date and the time"
        ));
    }
    let start = end + 1;
    let (time, end) = time_prefix(&bytes[start..])?;
    let end = start + end;
    let rest = &text[end..];
    let offset = if rest == "Z" {
        Some(Offset::UTC)
    } else if rest.len() == 6 && rest.as_bytes()[3] == b':' {
        parse_offset(rest)
    } else {
        None
    };
    let Some(offset) = offset else {
        return Err(format!(
            "expected `Z` or an offset such as `+02:00` at byte {end}, in the form `{FORM}`"
        ));
    };
    offset
        .to_timestamp(date.to_datetime(time))
        .map_err(|_| PAST_THE_RANGE.to_owned())
}

/// The date `text` names: `Date::fromIso`'s `YYYY-MM-DD`, with a `-` before
/// the year for a year before year 1.
///
/// # Errors
///
/// The sentence `Date::fromIso` throws with.
pub fn date(text: &str) -> Result<civil::Date, String> {
    let bytes = text.as_bytes();
    let (date, end) = date_prefix(bytes)?;
    rest(bytes, end, "YYYY-MM-DD")?;
    Ok(date)
}

/// The time of day `text` names: `TimeOfDay::fromIso`'s `HH:MM`, `HH:MM:SS`
/// or `HH:MM:SS.f` with one to nine fraction digits.
///
/// # Errors
///
/// The sentence `TimeOfDay::fromIso` throws with.
pub fn time_of_day(text: &str) -> Result<civil::Time, String> {
    let bytes = text.as_bytes();
    let (time, end) = time_prefix(bytes)?;
    rest(bytes, end, "HH:MM:SS")?;
    Ok(time)
}

/// What a `DateTime::fromIso` text names: the local date and time, and the
/// offset and the zone where the text writes them.
#[derive(Debug)]
pub struct Local {
    /// The date and the time of day, midnight for a date alone.
    pub civil: civil::DateTime,
    /// The `±HH:MM[:SS]` before the zone, which picks one copy of a repeated
    /// hour.
    pub offset: Option<Offset>,
    /// The zone in the brackets, as its id and as the zone it names.
    pub zone: Option<(String, TimeZone)>,
}

/// The local date and time `text` names, and its offset and zone where it
/// writes them: `DateTime::fromIso`'s grammar.
///
/// A date alone is midnight on it, and the caller resolves midnight in a zone.
/// After the time the text may write a `±HH:MM` or `±HH:MM:SS` offset and then
/// one `[Zone]`, as RFC 9557 writes a zoned time. An offset needs the zone after
/// it, and a `Z` is refused: such a text names an instant, and the sentence
/// names the member that reads one. A zone name is looked up as `Zone::of` looks
/// one up, ignoring case, and an offset in the brackets is a fixed zone.
///
/// # Errors
///
/// The sentence `DateTime::fromIso` throws with.
pub fn local(text: &str) -> Result<Local, String> {
    const FORM: &str = "YYYY-MM-DDTHH:MM:SS+HH:MM[Zone]";
    let bytes = text.as_bytes();
    let (date, end) = date_prefix(bytes)?;
    let (civil, end) = if end == bytes.len() || bytes[end] == b'[' {
        (date.to_datetime(civil::Time::midnight()), end)
    } else {
        if !matches!(bytes[end], b'T' | b' ') {
            return Err(format!(
                "expected `T` or a space at byte {end}, between the date and the time"
            ));
        }
        let start = end + 1;
        let (time, end) = time_prefix(&bytes[start..])?;
        (date.to_datetime(time), start + end)
    };
    if matches!(bytes.get(end), Some(b'Z' | b'z')) {
        return Err("the text has an offset `Z` and no zone. Read it with \
             `Core\\Time\\Instant::fromIso`, then call `->in($zone)`"
            .to_owned());
    }
    let mut at = end;
    let offset = if matches!(bytes.get(at), Some(b'+' | b'-')) {
        let width = if bytes.get(at + 6) == Some(&b':') {
            9
        } else {
            6
        };
        let written = text.get(at..at + width).filter(|written| {
            written.as_bytes()[1..].iter().enumerate().all(|(i, byte)| {
                if i % 3 == 2 {
                    *byte == b':'
                } else {
                    byte.is_ascii_digit()
                }
            })
        });
        let Some(offset) = written.and_then(parse_offset) else {
            return Err(format!(
                "the offset at byte {at} is written `+HH:MM` or `+HH:MM:SS`, in the form `{FORM}`"
            ));
        };
        at += width;
        if bytes.get(at) != Some(&b'[') {
            return Err(
                "the text has an offset and no zone. Write the zone after the offset, \
                        such as `[Europe/Vienna]`, or read the text with \
                        `Core\\Time\\Instant::fromIso`, then call `->in($zone)`"
                    .to_owned(),
            );
        }
        Some(offset)
    } else {
        None
    };
    let zone = if bytes.get(at) == Some(&b'[') {
        let Some(close) = text[at..].find(']').map(|close| at + close) else {
            return Err(format!("the zone at byte {at} has no closing `]`"));
        };
        let name = &text[at + 1..close];
        if name.starts_with('!') {
            return Err(
                "a critical flag `[!...]` is not read. Write the zone name alone".to_owned(),
            );
        }
        if name.contains('=') {
            return Err(format!(
                "`[{}]` is a tag, and only a zone is read in brackets",
                crate::quoted(name)
            ));
        }
        let id = written_zone(name)?;
        let zone = resolve_zone(&id)
            .ok_or_else(|| format!("`{}` is not a time zone name", crate::quoted(name)))?;
        at = close + 1;
        Some((id, zone))
    } else {
        None
    };
    rest(bytes, at, FORM)?;
    Ok(Local {
        civil,
        offset,
        zone,
    })
}

/// The zoned time `text` names where the text writes its own `[Zone]`: the
/// `DateTime` a route, a command line and JSON read, none of which has a
/// `$zone` to fall back on.
///
/// # Errors
///
/// [`local`]'s sentence, the sentence for a text with no zone, [`place`]'s
/// sentence for an offset the zone does not have, or [`PAST_THE_RANGE`].
pub fn zoned(text: &str) -> Result<Zoned, String> {
    let read = local(text)?;
    let Some((_, zone)) = &read.zone else {
        return Err("the text names no zone. Write one after the time, such as \
                    `[Europe/Vienna]`"
            .to_owned());
    };
    place(read.civil, read.offset, zone).map_err(|why| match why {
        Placed::Offset(why) => why,
        Placed::Range => PAST_THE_RANGE.to_owned(),
    })
}

/// Why [`place`] could not place a local time.
#[derive(Debug)]
pub enum Placed {
    /// The offset is not one the zone has at that local time. The sentence
    /// names both.
    Offset(String),
    /// The moment is past either end of the range.
    Range,
}

/// `civil` placed in `zone`. Without an offset, a skipped local time moves
/// forward past the gap and a repeated one is the earlier copy, as
/// `DateTime::at` places it. With one, the offset names the moment, and the
/// zone must have that offset at that moment: so an offset picks one copy of a
/// repeated hour and refuses a skipped time.
///
/// # Errors
///
/// [`Placed`]'s two reasons.
pub fn place(
    civil: civil::DateTime,
    offset: Option<Offset>,
    zone: &TimeZone,
) -> Result<Zoned, Placed> {
    let Some(offset) = offset else {
        return zone.to_zoned(civil).map_err(|_| Placed::Range);
    };
    let at = offset.to_timestamp(civil).map_err(|_| Placed::Range)?;
    let had = zone.to_offset(at);
    if had != offset {
        return Err(Placed::Offset(format!(
            "the zone has no offset `{}` at {}. Its offset at that moment is `{}`",
            render_offset(offset.seconds()),
            date_text(civil.date()) + "T" + &clock_text(civil.time()),
            render_offset(had.seconds())
        )));
    }
    Ok(Zoned::new(at, zone.clone()))
}

/// `date` as `YYYY-MM-DD`, with the year in `fromIso`'s one form: four digits,
/// and a `-` before them for a year before year 1.
#[must_use]
pub fn date_text(date: civil::Date) -> String {
    let year = i32::from(date.year());
    let sign = if year < 0 { "-" } else { "" };
    format!(
        "{sign}{:04}-{:02}-{:02}",
        year.unsigned_abs(),
        date.month(),
        date.day()
    )
}

/// `time` as `HH:MM:SS`, always with seconds, and a fraction with as few digits
/// as keep it exact when it is not zero.
#[must_use]
pub fn clock_text(time: civil::Time) -> String {
    let mut text = format!(
        "{:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
    let nanos = time.subsec_nanosecond();
    if nanos != 0 {
        let fraction = format!("{nanos:09}");
        text.push('.');
        text.push_str(fraction.trim_end_matches('0'));
    }
    text
}

/// The id a zone written as text is stored under: the name the database
/// writes for an IANA name, and `±HH:MM[:SS]` for an offset.
///
/// A name is looked up with `TimeZone::get`, which ignores case, and is stored
/// as the database writes it, so one zone has one id however the text wrote it.
/// An offset is read by [`parse_offset`] and written back by [`render_offset`],
/// so `+2:00` and `+02:00` are one id.
///
/// # Errors
///
/// The sentence for text that names no zone.
pub fn written_zone(text: &str) -> Result<String, String> {
    if text.starts_with('+') || text.starts_with('-') {
        return parse_offset(text)
            .map(|offset| render_offset(offset.seconds()))
            .ok_or_else(|| {
                format!(
                    "`{}` is not a zone offset. An offset is written `+HH:MM` and is at \
                     most 25:59:59 from UTC",
                    crate::quoted(text)
                )
            });
    }
    if let Some(name) = database_name(text) {
        return Ok(name);
    }
    Err(format!("`{}` is not a time zone name", crate::quoted(text)))
}

/// The [`TimeZone`] an id names, or `None` for one nothing does.
///
/// No IANA identifier begins with a sign, so the first byte decides whether an
/// id is an offset or a name.
#[must_use]
pub fn resolve_zone(id: &str) -> Option<TimeZone> {
    if id.starts_with('+') || id.starts_with('-') {
        return parse_offset(id).map(TimeZone::fixed);
    }
    if id.len() > ZONE_NAME_MAX {
        return None;
    }
    TimeZone::get(id).ok()
}

/// The name the zone database writes for the zone `text` names, or `None`
/// for a name it does not have, or one longer than [`ZONE_NAME_MAX`].
///
/// The lookup ignores ASCII case, so `europe/berlin` names a zone, and the
/// zone keeps the database's `Europe/Berlin`.
#[must_use]
pub fn database_name(text: &str) -> Option<String> {
    if text.len() > ZONE_NAME_MAX {
        return None;
    }
    let zone = TimeZone::get(text).ok()?;
    Some(zone.iana_name().unwrap_or(text).to_owned())
}

/// A `±HH:MM[:SS]` offset, or `None` for anything else.
///
/// One sign, then digits alone: one or two for the hour, and exactly two for
/// the minute and the second. A field is never read with `str::parse`, which
/// takes a sign of its own, so `+-5:00` is refused and not read as `-05:00`.
#[must_use]
pub fn parse_offset(text: &str) -> Option<Offset> {
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let field = |part: Option<&str>, widths: std::ops::RangeInclusive<usize>| -> Option<i32> {
        let part = part?;
        if !widths.contains(&part.len()) || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        Some(
            part.bytes()
                .fold(0, |sum, digit| sum * 10 + i32::from(digit - b'0')),
        )
    };
    let mut fields = text[1..].split(':');
    let hours = field(fields.next(), 1..=2)?;
    let minutes = field(fields.next(), 2..=2)?;
    let seconds = match fields.next() {
        Some(part) => field(Some(part), 2..=2)?,
        None => 0,
    };
    if fields.next().is_some() || minutes >= 60 || seconds >= 60 {
        return None;
    }
    Offset::from_seconds(sign * (hours * 3_600 + minutes * 60 + seconds)).ok()
}

/// The `±HH:MM[:SS]` text of a whole-second offset: [`parse_offset`]'s
/// inverse, and the id a fixed zone is stored under.
#[must_use]
pub fn render_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let total = seconds.unsigned_abs();
    let (hours, minutes, seconds) = (total / 3_600, (total % 3_600) / 60, total % 60);
    if seconds == 0 {
        format!("{sign}{hours:02}:{minutes:02}")
    } else {
        format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
    }
}

/// The three civil fields narrowed to the widths [`civil::Date::new`] takes,
/// or `None` for a number no calendar field can hold.
#[must_use]
pub fn parts(year: i64, month: i64, day: i64) -> Option<(i16, i8, i8)> {
    Some((
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    ))
}

/// The four fields as a [`civil::Time`], or `None` for a combination that is
/// not one, including a number no clock field can hold.
#[must_use]
pub fn clock_from_parts(hour: i64, minute: i64, second: i64, nanos: i64) -> Option<civil::Time> {
    let hour = i8::try_from(hour).ok()?;
    let minute = i8::try_from(minute).ok()?;
    let second = i8::try_from(second).ok()?;
    let nanos = i32::try_from(nanos).ok()?;
    civil::Time::new(hour, minute, second, nanos).ok()
}

/// A run of exactly `width` ASCII digits at `at`, as a number, or the sentence
/// naming `what` and the form it is written in.
fn digits(bytes: &[u8], at: usize, width: usize, what: &str, form: &str) -> Result<i64, String> {
    let run = bytes
        .get(at..at + width)
        .filter(|run| run.iter().all(u8::is_ascii_digit));
    let Some(run) = run else {
        return Err(format!(
            "the {what} is {width} digits, in the form `{form}`"
        ));
    };
    Ok(run
        .iter()
        .fold(0, |sum, digit| sum * 10 + i64::from(digit - b'0')))
}

/// `byte` at `at`, or the sentence naming what was expected there.
fn expect(bytes: &[u8], at: usize, byte: u8, form: &str) -> Result<(), String> {
    if bytes.get(at) == Some(&byte) {
        Ok(())
    } else {
        Err(format!(
            "expected `{}` at byte {at}, in the form `{form}`",
            char::from(byte)
        ))
    }
}

/// The date at the start of `bytes` and where it ends.
fn date_prefix(bytes: &[u8]) -> Result<(civil::Date, usize), String> {
    const FORM: &str = "YYYY-MM-DD";
    let signed = bytes.first() == Some(&b'-');
    let start = usize::from(signed);
    let year = digits(bytes, start, 4, "year", FORM)?;
    expect(bytes, start + 4, b'-', FORM)?;
    let month = digits(bytes, start + 5, 2, "month", FORM)?;
    expect(bytes, start + 7, b'-', FORM)?;
    let day = digits(bytes, start + 8, 2, "day", FORM)?;
    // Year zero is written `0000`, and one value has one text.
    if signed && year == 0 {
        return Err("the year `-0000` is written `0000`".to_owned());
    }
    let year = if signed { -year } else { year };
    if !(1..=12).contains(&month) {
        return Err(format!(
            "the month is `{month:02}`, and a month is `01` to `12`"
        ));
    }
    let date = parts(year, month, day)
        .and_then(|(year, month, day)| civil::Date::new(year, month, day).ok())
        .ok_or_else(|| format!("{year:04}-{month:02} has no day {day:02}"))?;
    Ok((date, start + 10))
}

/// The time of day at the start of `bytes` and where it ends.
fn time_prefix(bytes: &[u8]) -> Result<(civil::Time, usize), String> {
    const FORM: &str = "HH:MM:SS";
    let hour = digits(bytes, 0, 2, "hour", FORM)?;
    expect(bytes, 2, b':', FORM)?;
    let minute = digits(bytes, 3, 2, "minute", FORM)?;
    let mut end = 5;
    let mut second = 0;
    let mut nanos = 0;
    if bytes.get(end) == Some(&b':') {
        second = digits(bytes, end + 1, 2, "second", FORM)?;
        end += 3;
        if bytes.get(end) == Some(&b'.') {
            let count = bytes[end + 1..]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if !(1..=9).contains(&count) {
                return Err("the fraction of a second is one to nine digits".to_owned());
            }
            let fraction = digits(bytes, end + 1, count, "fraction", FORM)?;
            let scale = 10_i64.pow(u32::try_from(9 - count).unwrap_or(0));
            nanos = fraction * scale;
            end += 1 + count;
        }
    }
    if hour > 23 {
        return Err(format!(
            "the hour is `{hour:02}`, and an hour is `00` to `23`"
        ));
    }
    if minute > 59 {
        return Err(format!(
            "the minute is `{minute:02}`, and a minute is `00` to `59`"
        ));
    }
    if second > 59 {
        return Err(format!(
            "the second is `{second:02}`, and a second is `00` to `59`"
        ));
    }
    let time = clock_from_parts(hour, minute, second, nanos)
        .ok_or_else(|| format!("`{hour:02}:{minute:02}:{second:02}` is not a time of day"))?;
    Ok((time, end))
}

/// The sentence for text left after a whole value was read.
fn rest(bytes: &[u8], at: usize, form: &str) -> Result<(), String> {
    if at == bytes.len() {
        Ok(())
    } else {
        Err(format!(
            "unexpected text at byte {at}, after the `{form}` value"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`ZONE_NAME_MAX`] refuses a name before the lookup, so it must be
    /// longer than every name the database has.
    #[test]
    fn every_zone_name_fits_the_bound() {
        for name in jiff::tz::db().available() {
            assert!(
                name.as_str().len() <= ZONE_NAME_MAX,
                "`{}` is longer than the bound",
                name.as_str()
            );
        }
    }

    /// A bracket longer than the bound is refused with the sentence for a name
    /// the database does not have.
    #[test]
    fn a_zone_name_past_the_bound_is_not_looked_up() {
        let long = "A".repeat(ZONE_NAME_MAX + 1);
        assert!(database_name(&long).is_none());
        assert!(resolve_zone(&long).is_none());
        assert!(written_zone(&long).is_err());
    }

    /// Each reader reads its own form and refuses the next class's.
    #[test]
    fn each_reader_reads_its_own_form() {
        assert!(date("2026-01-05").is_ok());
        assert!(date("2026-01-05T08:30").is_err());
        assert!(time_of_day("08:30").is_ok());
        assert!(time_of_day("2026-01-05").is_err());
        assert!(instant("2026-01-05T08:30:00Z").is_ok());
        assert!(instant("2026-01-05T08:30:00").is_err());
        assert!(zoned("2026-01-05T08:30:00[Europe/Vienna]").is_ok());
        assert!(zoned("2026-01-05T08:30:00").is_err());
    }
}
