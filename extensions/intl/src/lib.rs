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
//! `sort-keys` (`collation`'s module doc), `format-numbers` (`numbers`'s) and `plural-categories`
//! (`plurals`'s); every other export returns `runtime`.
//!
//! A locale is a BCP 47 tag that [`locale`] parses for every export. A malformed tag is
//! `Invalid` and names the tag. A well-formed tag with no data of its own falls back along CLDR's
//! chain inside ICU4X, down to the root locale.

pub mod collation;
pub mod numbers;
pub mod plurals;

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
        Guest, Length, ListOptions, LocalDateTime, NumberOptions, NumberStyle, PluralCategory,
        PluralKind, RelativeItem, RelativeOptions, Service, Strength, TimeOfDay, TimeOptions, Word,
    };

    struct Component;

    fn error(err: crate::Error) -> Error {
        match err {
            crate::Error::Invalid(message) => Error::Invalid(message),
            crate::Error::Parse(message) => Error::Parse(message),
            crate::Error::Runtime(message) => Error::Runtime(message),
        }
    }

    fn not_implemented<T>(export: &str) -> Result<T, Error> {
        Err(Error::Runtime(format!(
            "`{export}` is not available in this version of `Novis\\Intl`."
        )))
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
            _values: Vec<LocalDateTime>,
            _locale: String,
            _options: DateTimeOptions,
        ) -> Result<Vec<String>, Error> {
            not_implemented("format-date-times")
        }

        fn format_dates(
            _values: Vec<Date>,
            _locale: String,
            _length: Length,
        ) -> Result<Vec<String>, Error> {
            not_implemented("format-dates")
        }

        fn format_times(
            _values: Vec<TimeOfDay>,
            _locale: String,
            _options: TimeOptions,
        ) -> Result<Vec<String>, Error> {
            not_implemented("format-times")
        }

        fn format_relative(
            _items: Vec<RelativeItem>,
            _locale: String,
            _options: RelativeOptions,
        ) -> Result<Vec<String>, Error> {
            not_implemented("format-relative")
        }

        fn format_lists(
            _lists: Vec<Vec<String>>,
            _locale: String,
            _options: ListOptions,
        ) -> Result<Vec<String>, Error> {
            not_implemented("format-lists")
        }

        fn word_segments(_strings: Vec<String>, _locale: String) -> Result<Vec<Vec<Word>>, Error> {
            not_implemented("word-segments")
        }

        fn sentence_segments(
            _strings: Vec<String>,
            _locale: String,
        ) -> Result<Vec<Vec<String>>, Error> {
            not_implemented("sentence-segments")
        }

        fn resolve_locales(_tags: Vec<String>, _service: Service) -> Result<Vec<String>, Error> {
            not_implemented("resolve-locales")
        }

        fn negotiate(
            _accept_language: String,
            _offered: Vec<String>,
            _default: String,
        ) -> Result<String, Error> {
            not_implemented("negotiate")
        }
    }

    export!(Component);
}
