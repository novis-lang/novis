//! Dates and times: `format-date-times`, `format-dates` and `format-times` over `icu_datetime`
//! (`rule:core-classes/intl-host-converts-time`, `rule:core-classes/intl-batch-shape`).
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
//! keyword in the tag selects another. The date parts are the [`Fields`] asked for, year, month
//! and day by default, at the given [`Length`]; the time is hours and minutes, and seconds only
//! when asked for. Nanoseconds are never shown. One formatter is built per call, and a field out
//! of its range is `Invalid`.
//!
//! A [`Fields`] case is one of ICU4X's `DateFields`, so the locale's data decides the order of
//! the parts and the words between them: there is no skeleton string to write
//! (`rule:core-api/no-mode-strings`). `Month`, `YearMonth` and `Year` are ICU4X's calendar
//! periods, which have no day and so no time beside them: `format-date-times` returns
//! `Invalid` for them before it formats anything, and a program formats the `Date` instead.

use icu_calendar::Iso;
use icu_datetime::fieldsets::T;
use icu_datetime::fieldsets::builder::{DateFields, FieldSetBuilder, ZoneStyle as Zone};
use icu_datetime::input::{Date as IsoDate, DateTime, Time, TimeZone, UtcOffset, ZonedDateTime};
use icu_datetime::options::{Length as IcuLength, TimePrecision};
use icu_datetime::pattern::DateTimeNames;
use icu_datetime::{DateTimeFormatter, NoCalendarFormatter};

use crate::Error;

/// How much of a date is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    Short,
    Medium,
    Long,
}

/// Which parts of a date are written, named from the largest to the smallest. The locale decides
/// their order and the words between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fields {
    Day,
    MonthDay,
    YearMonthDay,
    Weekday,
    DayWeekday,
    MonthDayWeekday,
    YearMonthDayWeekday,
    Month,
    YearMonth,
    Year,
}

impl Fields {
    /// Every case, in the order WIT declares them.
    pub const ALL: [Self; 10] = [
        Self::Day,
        Self::MonthDay,
        Self::YearMonthDay,
        Self::Weekday,
        Self::DayWeekday,
        Self::MonthDayWeekday,
        Self::YearMonthDayWeekday,
        Self::Month,
        Self::YearMonth,
        Self::Year,
    ];

    fn icu(self) -> DateFields {
        match self {
            Self::Day => DateFields::D,
            Self::MonthDay => DateFields::MD,
            Self::YearMonthDay => DateFields::YMD,
            Self::Weekday => DateFields::E,
            Self::DayWeekday => DateFields::DE,
            Self::MonthDayWeekday => DateFields::MDE,
            Self::YearMonthDayWeekday => DateFields::YMDE,
            Self::Month => DateFields::M,
            Self::YearMonth => DateFields::YM,
            Self::Year => DateFields::Y,
        }
    }

    /// Whether the case has a day, which a time can be written beside.
    #[must_use]
    pub fn has_day(self) -> bool {
        !matches!(self, Self::Month | Self::YearMonth | Self::Year)
    }
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

/// The options of `format-dates`. An absent field is the default: `Medium` and `YearMonthDay`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateOptions {
    pub length: Option<Length>,
    pub fields: Option<Fields>,
}

/// The options of `format-date-times`. An absent field is the default: `Medium`,
/// `YearMonthDay`, no seconds and no zone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateTimeOptions {
    pub length: Option<Length>,
    pub fields: Option<Fields>,
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
/// `Invalid` for a malformed tag, a field out of its range, an offset no zone has and [`Fields`]
/// with no day, and `Runtime` when the locale has no date data at all.
pub fn format_date_times(
    values: &[LocalDateTime],
    tag: &str,
    options: DateTimeOptions,
) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let fields = options.fields.unwrap_or(Fields::YearMonthDay);
    if !fields.has_day() {
        return Err(Error::Invalid(format!(
            "`Fields::{fields:?}` shows no day, so it cannot be shown with a time. \
             Format the date with `DateFormat::dates` instead."
        )));
    }
    let mut builder = FieldSetBuilder::new();
    builder.length = Some(length(options.length.unwrap_or(Length::Medium)));
    builder.date_fields = Some(fields.icu());
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
    let formatter =
        DateTimeFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
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

/// Each date written in `tag`'s locale under `options`.
///
/// # Errors
///
/// `Invalid` for a malformed tag or a field out of its range, and `Runtime` when the locale has
/// no date data at all.
pub fn format_dates(
    values: &[Date],
    tag: &str,
    options: DateOptions,
) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let fields = options.fields.unwrap_or(Fields::YearMonthDay);
    let mut builder = FieldSetBuilder::new();
    builder.length = Some(length(options.length.unwrap_or(Length::Medium)));
    builder.date_fields = Some(fields.icu());
    let no_format = |err| Error::Runtime(format!("No date format for these options: {err}."));
    // A date field set and a calendar period are two formatter types, so each has its arm.
    if fields.has_day() {
        let field_set = builder.build_date().map_err(no_format)?;
        let formatter =
            DateTimeFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
        values
            .iter()
            .map(|value| Ok(formatter.format(&iso_date(*value)?).to_string()))
            .collect()
    } else {
        let field_set = builder.build_calendar_period().map_err(no_format)?;
        let formatter =
            DateTimeFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
        // ICU4X 2.4 does not load the number formatter for some calendar-period patterns, such
        // as `pt`'s long year and month, and then writes the year in ASCII digits as an error
        // part. Loading it here writes the locale's own digits. It is loaded once per call.
        let mut names = DateTimeNames::from_formatter((&locale).into(), formatter);
        names
            .as_mut()
            .include_decimal_formatter()
            .map_err(|err| Error::Runtime(format!("No number data for `{tag}`: {err}.")))?;
        let formatter = names
            .try_into_formatter(field_set)
            .map_err(|(err, _)| no_data(tag)(err))?;
        values
            .iter()
            .map(|value| Ok(formatter.format(&iso_date(*value)?).to_string()))
            .collect()
    }
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
    let formatter =
        NoCalendarFormatter::try_new((&locale).into(), field_set).map_err(no_data(tag))?;
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
        assert_eq!(
            place.unwrap(),
            ["Jul 14, 2026, 3:05\u{202f}PM Austria Time"]
        );
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
        let long = DateOptions {
            length: Some(Length::Long),
            ..DateOptions::default()
        };
        assert_eq!(
            format_dates(&[date], "en", long).unwrap(),
            ["March 1, 2026"]
        );
        let time = TimeOfDay {
            hour: 9,
            minute: 30,
            second: 15,
            nanos: 0,
        };
        assert_eq!(
            format_times(&[time], "en-GB", Some(true)).unwrap(),
            ["09:30:15"]
        );
    }

    #[test]
    fn a_field_out_of_range_is_invalid() {
        let date = Date {
            year: 2026,
            month: 13,
            day: 1,
        };
        let short = DateOptions {
            length: Some(Length::Short),
            ..DateOptions::default()
        };
        assert!(matches!(
            format_dates(&[date], "en", short),
            Err(Error::Invalid(_))
        ));
    }

    fn fields(fields: Fields) -> DateOptions {
        DateOptions {
            length: Some(Length::Long),
            fields: Some(fields),
        }
    }

    /// Every locale the built-in date data has its own weekday names for, which is the set
    /// `negotiate`'s `Dates` service reads. The baked provider does not list its locales, so the
    /// data crate's macro lists them for a probe type of this test's own.
    fn every_dates_locale() -> Vec<String> {
        use icu_datetime_data::__impl_datetime_names_weekday_v1;
        use icu_provider::IterableDataProvider;
        mod icu {
            pub use icu_datetime as datetime;
        }
        struct Probe;
        icu_datetime_data::make_provider!(Probe);
        icu_datetime_data::impl_datetime_names_weekday_v1!(Probe, ITER);
        let ids =
            IterableDataProvider::<icu_datetime::provider::names::DatetimeNamesWeekdayV1>::iter_ids(
                &Probe,
            )
            .unwrap();
        let mut tags: Vec<String> = ids.iter().map(|id| id.locale.to_string()).collect();
        tags.sort();
        tags.dedup();
        tags
    }

    #[test]
    fn date_format_fields_every_case_formats_in_every_locale() {
        let date = Date {
            year: 2026,
            month: 7,
            day: 14,
        };
        let locales = every_dates_locale();
        assert!(
            locales.len() > 100,
            "only {} locales were listed",
            locales.len()
        );
        // A formatter that misses data panics in a debug build, so a panic is a failure too,
        // and every failing case and locale is named at once.
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let mut failed = Vec::new();
        let mut judge = |what: String, run: &dyn Fn() -> Result<Vec<String>, Error>| {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
                Ok(Ok(text)) if !text[0].trim().is_empty() => {}
                Ok(Ok(_)) => failed.push(format!("{what}: formats nothing")),
                Ok(Err(err)) => failed.push(format!("{what}: {err:?}")),
                Err(_) => failed.push(format!("{what}: panics")),
            }
        };
        for tag in &locales {
            for case in Fields::ALL {
                for at in [Length::Short, Length::Medium, Length::Long] {
                    let options = DateOptions {
                        length: Some(at),
                        fields: Some(case),
                    };
                    judge(format!("{case:?} at {at:?} in `{tag}`"), &|| {
                        format_dates(&[date], tag, options)
                    });
                }
                if case.has_day() {
                    let options = DateTimeOptions {
                        fields: Some(case),
                        ..DateTimeOptions::default()
                    };
                    judge(format!("{case:?} with a time in `{tag}`"), &|| {
                        format_date_times(&[vienna(7, 15, 7200)], tag, options)
                    });
                }
            }
        }
        std::panic::set_hook(quiet);
        assert!(
            failed.is_empty(),
            "{} failed:\n{}",
            failed.len(),
            failed.join("\n")
        );
    }

    #[test]
    fn date_format_fields_year_month_in_de() {
        let date = Date {
            year: 2026,
            month: 1,
            day: 5,
        };
        let text = |case| format_dates(&[date], "de", fields(case)).unwrap();
        assert_eq!(text(Fields::YearMonth), ["Januar 2026"]);
        assert_eq!(text(Fields::Month), ["Januar"]);
        assert_eq!(text(Fields::Year), ["2026"]);
        assert_eq!(text(Fields::MonthDay), ["5. Januar"]);
        assert_eq!(text(Fields::YearMonthDay), ["5. Januar 2026"]);
    }

    #[test]
    fn date_format_fields_weekday_in_de() {
        let date = Date {
            year: 2026,
            month: 1,
            day: 5,
        };
        let text = |case| format_dates(&[date], "de", fields(case)).unwrap();
        assert_eq!(text(Fields::Weekday), ["Montag"]);
        assert_eq!(
            text(Fields::YearMonthDayWeekday),
            ["Montag, 5. Januar 2026"]
        );
    }

    #[test]
    fn date_format_fields_on_date_times() {
        let options = |case| DateTimeOptions {
            fields: Some(case),
            ..DateTimeOptions::default()
        };
        let values = [vienna(7, 15, 7200)];
        let weekday = format_date_times(&values, "en", options(Fields::Weekday)).unwrap();
        assert_eq!(weekday, ["Tue 3:05\u{202f}PM"]);
        for case in [Fields::Month, Fields::YearMonth, Fields::Year] {
            let refused = format_date_times(&values, "en", options(case));
            assert!(
                matches!(refused, Err(Error::Invalid(_))),
                "{case:?}: {refused:?}"
            );
        }
    }
}
