//! Dates and times: `format-date-times`, `format-dates` and `format-times` over `icu_datetime`
//! (ADR 0277 § 6 and § 9, `rule:core-classes/intl-batch-shape`).
//!
//! The guest carries no time-zone database. The host places a `Core\Time\DateTime` in its zone
//! and sends the local ISO fields, the UTC offset in seconds and the IANA zone id; the guest
//! formats those fields and never computes an offset. The zone id is read only to name the zone:
//! [`ZoneStyle::Offset`] writes the offset the host sent ("GMT+2"), [`ZoneStyle::Location`] the
//! zone's place, and [`ZoneStyle::Generic`] its generic long name ("Central European Time"). An id
//! the guest does not know is named by its offset. The generic name of a zone that changed
//! metazone is chosen by the local date and offset, which is metazone data and not a zone's
//! transitions.
//!
//! The fields cross as ISO fields and are formatted in the locale's calendar, so a `-u-ca-`
//! keyword in the tag selects another. The date fields are year, month and day at the given
//! [`Length`]; the time is hours and minutes, and seconds only when asked for. Nanoseconds are
//! never shown. One formatter is built per call, and a field out of its range is `Invalid`.

use icu_calendar::Iso;
use icu_datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle as Zone};
use icu_datetime::fieldsets::{T, YMD};
use icu_datetime::input::{Date as IsoDate, DateTime, Time, TimeZone, UtcOffset, ZonedDateTime};
use icu_datetime::options::{Length as IcuLength, TimePrecision};
use icu_datetime::{DateTimeFormatter, NoCalendarFormatter};

use crate::Error;

/// How much of a date is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    Short,
    Medium,
    Long,
}

/// How the zone of a date and time is named, if at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneStyle {
    None,
    Offset,
    Location,
    Generic,
}

/// A date and time of day as the host computed it in its zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDateTime {
    pub date: Date,
    pub time: TimeOfDay,
    pub offset_seconds: i64,
    pub zone: String,
}

/// An ISO calendar date, `month` and `day` from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub nanos: i64,
}

/// The options of `format-date-times`. An absent field is the default: `Medium`, no seconds and
/// no zone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateTimeOptions {
    pub length: Option<Length>,
    pub seconds: Option<bool>,
    pub zone: Option<ZoneStyle>,
}

fn length(length: Length) -> IcuLength {
    match length {
        Length::Short => IcuLength::Short,
        Length::Medium => IcuLength::Medium,
        Length::Long => IcuLength::Long,
    }
}

fn precision(seconds: Option<bool>) -> TimePrecision {
    if seconds.unwrap_or(false) {
        TimePrecision::Second
    } else {
        TimePrecision::Minute
    }
}

fn no_data(tag: &str) -> impl Fn(icu_datetime::DateTimeFormatterLoadError) -> Error + '_ {
    move |err| Error::Runtime(format!("No date data for `{tag}`: {err}."))
}

fn iso_date(date: Date) -> Result<IsoDate<Iso>, Error> {
    let invalid = || {
        Error::Invalid(format!(
            "{}-{}-{} is not a valid date.",
            date.year, date.month, date.day
        ))
    };
    let year = i32::try_from(date.year).map_err(|_| invalid())?;
    let month = u8::try_from(date.month).map_err(|_| invalid())?;
    let day = u8::try_from(date.day).map_err(|_| invalid())?;
    IsoDate::try_new_iso(year, month, day).map_err(|_| invalid())
}

fn time(time: TimeOfDay) -> Result<Time, Error> {
    let invalid = || {
        Error::Invalid(format!(
            "{}:{}:{}.{} is not a valid time of day.",
            time.hour, time.minute, time.second, time.nanos
        ))
    };
    let hour = u8::try_from(time.hour).map_err(|_| invalid())?;
    let minute = u8::try_from(time.minute).map_err(|_| invalid())?;
    let second = u8::try_from(time.second).map_err(|_| invalid())?;
    let nanos = u32::try_from(time.nanos).map_err(|_| invalid())?;
    Time::try_new(hour, minute, second, nanos).map_err(|_| invalid())
}

/// Each value written in `tag`'s locale under `options`.
///
/// # Errors
///
/// `Invalid` for a malformed tag, a field out of its range or an offset no zone has, and
/// `Runtime` when the locale has no date data at all.
pub fn format_date_times(
    values: &[LocalDateTime],
    tag: &str,
    options: DateTimeOptions,
) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let mut builder = FieldSetBuilder::new();
    builder.length = Some(length(options.length.unwrap_or(Length::Medium)));
    builder.date_fields = Some(DateFields::YMD);
    builder.time_precision = Some(precision(options.seconds));
    builder.zone_style = match options.zone.unwrap_or(ZoneStyle::None) {
        ZoneStyle::None => None,
        ZoneStyle::Offset => Some(Zone::LocalizedOffsetShort),
        ZoneStyle::Location => Some(Zone::Location),
        ZoneStyle::Generic => Some(Zone::GenericLong),
    };
    let field_set = builder
        .build_composite()
        .map_err(|err| Error::Runtime(format!("No date format for these options: {err}.")))?;
    let formatter = DateTimeFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
    values
        .iter()
        .map(|value| {
            let date = iso_date(value.date)?;
            let time = time(value.time)?;
            let offset = i32::try_from(value.offset_seconds)
                .ok()
                .and_then(|seconds| UtcOffset::try_from_seconds(seconds).ok())
                .ok_or_else(|| {
                    Error::Invalid(format!(
                        "{} seconds is not a valid UTC offset.",
                        value.offset_seconds
                    ))
                })?;
            let zone = TimeZone::from_iana_id(&value.zone)
                .with_offset(Some(offset))
                .at_date_time(DateTime { date, time });
            Ok(formatter
                .format(&ZonedDateTime { date, time, zone })
                .to_string())
        })
        .collect()
}

/// Each date written in `tag`'s locale at `length`.
///
/// # Errors
///
/// `Invalid` for a malformed tag or a field out of its range, and `Runtime` when the locale has
/// no date data at all.
pub fn format_dates(values: &[Date], tag: &str, at: Length) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let formatter = DateTimeFormatter::try_new((&locale).into(), YMD::for_length(length(at)))
        .map_err(no_data(tag))?;
    values
        .iter()
        .map(|value| Ok(formatter.format(&iso_date(*value)?).to_string()))
        .collect()
}

/// Each time of day written in `tag`'s locale, with seconds only when `seconds` is `true`.
///
/// # Errors
///
/// `Invalid` for a malformed tag or a field out of its range, and `Runtime` when the locale has
/// no time data at all.
pub fn format_times(
    values: &[TimeOfDay],
    tag: &str,
    seconds: Option<bool>,
) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let field_set = if seconds.unwrap_or(false) {
        T::hms()
    } else {
        T::hm()
    };
    let formatter = NoCalendarFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
    values
        .iter()
        .map(|value| Ok(formatter.format(&time(*value)?).to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vienna(month: i64, hour: i64, offset_seconds: i64) -> LocalDateTime {
        LocalDateTime {
            date: Date {
                year: 2026,
                month,
                day: 14,
            },
            time: TimeOfDay {
                hour,
                minute: 5,
                second: 9,
                nanos: 0,
            },
            offset_seconds,
            zone: "Europe/Vienna".to_owned(),
        }
    }

    fn with_zone(zone: ZoneStyle) -> DateTimeOptions {
        DateTimeOptions {
            zone: Some(zone),
            ..DateTimeOptions::default()
        }
    }

    #[test]
    fn a_date_and_time_formats_by_locale() {
        let values = [vienna(7, 15, 7200)];
        let en = format_date_times(&values, "en", DateTimeOptions::default()).unwrap();
        assert_eq!(en, ["Jul 14, 2026, 3:05\u{202f}PM"]);
        let de = format_date_times(&values, "de", DateTimeOptions::default()).unwrap();
        assert_eq!(de, ["14.07.2026, 15:05"]);
    }

    #[test]
    fn the_offset_named_is_the_one_the_host_sent() {
        let summer = format_date_times(&[vienna(7, 15, 7200)], "en", with_zone(ZoneStyle::Offset));
        assert_eq!(summer.unwrap(), ["Jul 14, 2026, 3:05\u{202f}PM GMT+2"]);
        // A guest with a zone database would write `GMT+2` here too.
        let sent = format_date_times(&[vienna(7, 15, 3600)], "en", with_zone(ZoneStyle::Offset));
        assert_eq!(sent.unwrap(), ["Jul 14, 2026, 3:05\u{202f}PM GMT+1"]);
    }

    #[test]
    fn a_zone_is_named_by_its_place_or_its_generic_name() {
        let values = [vienna(7, 15, 7200)];
        let place = format_date_times(&values, "en", with_zone(ZoneStyle::Location));
        assert_eq!(place.unwrap(), ["Jul 14, 2026, 3:05\u{202f}PM Austria Time"]);
        let generic = format_date_times(&values, "en", with_zone(ZoneStyle::Generic));
        assert_eq!(
            generic.unwrap(),
            ["Jul 14, 2026, 3:05\u{202f}PM Central European Time"]
        );
        let unknown = LocalDateTime {
            zone: "Nowhere/Shop".to_owned(),
            ..vienna(7, 15, 7200)
        };
        let named = format_date_times(&[unknown], "en", with_zone(ZoneStyle::Generic));
        assert_eq!(named.unwrap(), ["Jul 14, 2026, 3:05\u{202f}PM GMT+02:00"]);
    }

    #[test]
    fn dates_and_times_alone() {
        let date = Date {
            year: 2026,
            month: 3,
            day: 1,
        };
        assert_eq!(format_dates(&[date], "en", Length::Long).unwrap(), ["March 1, 2026"]);
        let time = TimeOfDay {
            hour: 9,
            minute: 30,
            second: 15,
            nanos: 0,
        };
        assert_eq!(format_times(&[time], "en-GB", Some(true)).unwrap(), ["09:30:15"]);
    }

    #[test]
    fn a_field_out_of_range_is_invalid() {
        let date = Date {
            year: 2026,
            month: 13,
            day: 1,
        };
        assert!(matches!(format_dates(&[date], "en", Length::Short), Err(Error::Invalid(_))));
    }
}
