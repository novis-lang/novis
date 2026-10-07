//! The intl component: `nvs:intl/icu` from `wit/intl.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, ADR 0277).
//!
//! The crate has two halves, as the image component's does. The core is plain Rust over ICU4X
//! that builds for the host too, so its tests run under an ordinary `cargo test` in this
//! directory. The WIT glue in `guest` is compiled only for a wasm target, and maps each export
//! onto the core and the core's types onto WIT's.
//!
//! ICU4X is pinned at one version in `Cargo.toml`, with `compiled_data`: the data is baked into
//! the wasm, and nothing is read at run time. The implemented exports are `collate-order` and
//! `sort-keys` (`collation`'s module doc), `format-numbers` (`numbers`'s), `plural-categories`
//! (`plurals`'s), `format-date-times`, `format-dates` and `format-times` (`dates`'s),
//! `format-relative` (`relative`'s), `format-lists` (`lists`'s), and `word-segments` and
//! `sentence-segments` (`segments`'s), and `negotiate` and `resolve-locales` (`negotiate`'s). Every
//! export is implemented.
//!
//! A locale is a BCP 47 tag that [`locale`] parses for every export. A malformed tag is
//! `Invalid` and names the tag. A well-formed tag with no data of its own falls back along CLDR's
//! chain inside ICU4X, down to the root locale.

pub mod collation;
pub mod dates;
pub mod lists;
pub mod negotiate;
pub mod numbers;
pub mod plurals;
pub mod relative;
pub mod segments;

use icu_locale::Locale;

/// The WIT `error`: `Invalid`, `Parse` and `Runtime` throw `LogicError`, `ParseError` and
/// `RuntimeError` (`rule:packaging/a-guest-crash-throws`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    Parse(String),
    Runtime(String),
}

/// The locale `tag` writes, or `Invalid` naming it.
pub fn locale(tag: &str) -> Result<Locale, Error> {
    Locale::try_from_str(tag)
        .map_err(|_| Error::Invalid(format!("`{tag}` is not a valid locale tag.")))
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/intl.wit"],
        world: "nvs:intl/intl",
        generate_all,
    });

    use exports::nvs::intl::icu::{
        CaseFirst, CollateOptions, CompactDisplay, CurrencyDisplay, Date, DateTimeOptions, Error,
        Guest, Length, ListOptions, ListType, LocalDateTime, NumberOptions, NumberStyle, Numeric,
        PluralCategory, PluralKind, RelativeItem, RelativeOptions, Service, Strength, TimeOfDay,
        TimeOptions, TimeUnit, Width, Word, ZoneStyle,
    };

    struct Component;

    fn error(err: crate::Error) -> Error {
        match err {
            crate::Error::Invalid(message) => Error::Invalid(message),
            crate::Error::Parse(message) => Error::Parse(message),
            crate::Error::Runtime(message) => Error::Runtime(message),
        }
    }

    fn collate_options(options: &CollateOptions) -> crate::collation::Options {
        use crate::collation::Strength as S;
        crate::collation::Options {
            strength: options.strength.map(|strength| match strength {
                Strength::Primary => S::Primary,
                Strength::Secondary => S::Secondary,
                Strength::Tertiary => S::Tertiary,
                Strength::Quaternary => S::Quaternary,
                Strength::Identical => S::Identical,
            }),
            case_first: options.case_first.map(|case_first| match case_first {
                CaseFirst::Off => crate::collation::CaseFirst::Off,
                CaseFirst::Upper => crate::collation::CaseFirst::Upper,
                CaseFirst::Lower => crate::collation::CaseFirst::Lower,
            }),
            numeric: options.numeric,
            ignore_punctuation: options.ignore_punctuation,
        }
    }

    fn length(length: Length) -> crate::dates::Length {
        match length {
            Length::Short => crate::dates::Length::Short,
            Length::Medium => crate::dates::Length::Medium,
            Length::Long => crate::dates::Length::Long,
        }
    }

    fn width(width: Width) -> crate::relative::Width {
        match width {
            Width::Wide => crate::relative::Width::Wide,
            Width::Short => crate::relative::Width::Short,
            Width::Narrow => crate::relative::Width::Narrow,
        }
    }

    fn date(date: &Date) -> crate::dates::Date {
        crate::dates::Date {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }

    fn time_of_day(time: &TimeOfDay) -> crate::dates::TimeOfDay {
        crate::dates::TimeOfDay {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            nanos: time.nanos,
        }
    }

    fn number_options(options: NumberOptions) -> crate::numbers::Options {
        use crate::numbers::{CompactDisplay as C, CurrencyDisplay as D, Style as S};
        crate::numbers::Options {
            style: match options.style {
                NumberStyle::Decimal => S::Decimal,
                NumberStyle::Percent => S::Percent,
                NumberStyle::Currency => S::Currency,
                NumberStyle::Compact => S::Compact,
            },
            min_fraction_digits: options.min_fraction_digits,
            max_fraction_digits: options.max_fraction_digits,
            grouping: options.grouping,
            currency: options.currency,
            currency_display: options.currency_display.map(|display| match display {
                CurrencyDisplay::Symbol => D::Symbol,
                CurrencyDisplay::Narrow => D::Narrow,
                CurrencyDisplay::Name => D::Name,
            }),
            compact_display: options.compact_display.map(|display| match display {
                CompactDisplay::Short => C::Short,
                CompactDisplay::Long => C::Long,
            }),
        }
    }

    impl Guest for Component {
        fn collate_order(
            strings: Vec<String>,
            locale: String,
            options: CollateOptions,
        ) -> Result<Vec<u64>, Error> {
            crate::collation::order(&strings, &locale, &collate_options(&options)).map_err(error)
        }

        fn sort_keys(
            strings: Vec<String>,
            locale: String,
            options: CollateOptions,
        ) -> Result<Vec<Vec<u8>>, Error> {
            crate::collation::sort_keys(&strings, &locale, &collate_options(&options))
                .map_err(error)
        }

        fn format_numbers(
            numbers: Vec<String>,
            locale: String,
            options: NumberOptions,
        ) -> Result<Vec<String>, Error> {
            crate::numbers::format(&numbers, &locale, &number_options(options)).map_err(error)
        }

        fn plural_categories(
            numbers: Vec<String>,
            locale: String,
            kind: PluralKind,
        ) -> Result<Vec<PluralCategory>, Error> {
            use crate::plurals::{Category, Kind};
            let kind = match kind {
                PluralKind::Cardinal => Kind::Cardinal,
                PluralKind::Ordinal => Kind::Ordinal,
            };
            let categories = crate::plurals::categories(&numbers, &locale, kind).map_err(error)?;
            Ok(categories
                .into_iter()
                .map(|category| match category {
                    Category::Zero => PluralCategory::Zero,
                    Category::One => PluralCategory::One,
                    Category::Two => PluralCategory::Two,
                    Category::Few => PluralCategory::Few,
                    Category::Many => PluralCategory::Many,
                    Category::Other => PluralCategory::Other,
                })
                .collect())
        }

        fn format_date_times(
            values: Vec<LocalDateTime>,
            locale: String,
            options: DateTimeOptions,
        ) -> Result<Vec<String>, Error> {
            use crate::dates::ZoneStyle as Z;
            let values: Vec<crate::dates::LocalDateTime> = values
                .into_iter()
                .map(|value| crate::dates::LocalDateTime {
                    date: crate::dates::Date {
                        year: value.year,
                        month: value.month,
                        day: value.day,
                    },
                    time: crate::dates::TimeOfDay {
                        hour: value.hour,
                        minute: value.minute,
                        second: value.second,
                        nanos: value.nanos,
                    },
                    offset_seconds: value.offset_seconds,
                    zone: value.zone,
                })
                .collect();
            let options = crate::dates::DateTimeOptions {
                length: options.length.map(length),
                seconds: options.seconds,
                zone: options.zone.map(|zone| match zone {
                    ZoneStyle::None => Z::None,
                    ZoneStyle::Offset => Z::Offset,
                    ZoneStyle::Location => Z::Location,
                    ZoneStyle::Generic => Z::Generic,
                }),
            };
            crate::dates::format_date_times(&values, &locale, options).map_err(error)
        }

        fn format_dates(
            values: Vec<Date>,
            locale: String,
            at: Length,
        ) -> Result<Vec<String>, Error> {
            let values: Vec<crate::dates::Date> = values.iter().map(date).collect();
            crate::dates::format_dates(&values, &locale, length(at)).map_err(error)
        }

        fn format_times(
            values: Vec<TimeOfDay>,
            locale: String,
            options: TimeOptions,
        ) -> Result<Vec<String>, Error> {
            let values: Vec<crate::dates::TimeOfDay> = values.iter().map(time_of_day).collect();
            crate::dates::format_times(&values, &locale, options.seconds).map_err(error)
        }

        fn format_relative(
            items: Vec<RelativeItem>,
            locale: String,
            options: RelativeOptions,
        ) -> Result<Vec<String>, Error> {
            use crate::relative::{Numeric as N, Unit as U};
            let items: Vec<crate::relative::Item> = items
                .iter()
                .map(|item| crate::relative::Item {
                    count: item.count,
                    unit: match item.unit {
                        TimeUnit::Second => U::Second,
                        TimeUnit::Minute => U::Minute,
                        TimeUnit::Hour => U::Hour,
                        TimeUnit::Day => U::Day,
                        TimeUnit::Week => U::Week,
                        TimeUnit::Month => U::Month,
                        TimeUnit::Quarter => U::Quarter,
                        TimeUnit::Year => U::Year,
                    },
                })
                .collect();
            let options = crate::relative::Options {
                width: options.width.map(width),
                numeric: options.numeric.map(|numeric| match numeric {
                    Numeric::Always => N::Always,
                    Numeric::Auto => N::Auto,
                }),
            };
            crate::relative::format(&items, &locale, options).map_err(error)
        }

        fn format_lists(
            lists: Vec<Vec<String>>,
            locale: String,
            options: ListOptions,
        ) -> Result<Vec<String>, Error> {
            use crate::lists::ListType as L;
            let options = crate::lists::Options {
                list_type: options.type_.map(|list_type| match list_type {
                    ListType::And => L::And,
                    ListType::Or => L::Or,
                    ListType::Unit => L::Unit,
                }),
                width: options.width.map(width),
            };
            crate::lists::format(&lists, &locale, options).map_err(error)
        }

        fn word_segments(strings: Vec<String>, locale: String) -> Result<Vec<Vec<Word>>, Error> {
            let words = crate::segments::words(&strings, &locale).map_err(error)?;
            Ok(words
                .into_iter()
                .map(|segments| {
                    segments
                        .into_iter()
                        .map(|word| Word {
                            text: word.text,
                            word_like: word.word_like,
                        })
                        .collect()
                })
                .collect())
        }

        fn sentence_segments(
            strings: Vec<String>,
            locale: String,
        ) -> Result<Vec<Vec<String>>, Error> {
            crate::segments::sentences(&strings, &locale).map_err(error)
        }

        fn resolve_locales(tags: Vec<String>, service: Service) -> Result<Vec<String>, Error> {
            use crate::negotiate::Service as S;
            let service = match service {
                Service::Collation => S::Collation,
                Service::Numbers => S::Numbers,
                Service::Plurals => S::Plurals,
                Service::Dates => S::Dates,
                Service::RelativeTime => S::RelativeTime,
                Service::Lists => S::Lists,
                Service::Segmentation => S::Segmentation,
            };
            crate::negotiate::resolve(&tags, service).map_err(error)
        }

        fn negotiate(
            accept_language: String,
            offered: Vec<String>,
            default: String,
        ) -> Result<String, Error> {
            crate::negotiate::negotiate(&accept_language, &offered, &default).map_err(error)
        }
    }

    export!(Component);
}
