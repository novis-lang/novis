//! The intl component's `nvs:intl/icu`, called typed: `wit/intl.wit`'s types as `bindgen!` writes
//! them, and the conversion of each to and from its Novis value.

use self::bindings::exports::nvs::intl::icu::{
    CaseFirst, CollateOptions, CompactDisplay, CurrencyDisplay, DateTimeOptions, Length,
    ListOptions, ListType, LocalDateTime, NumberOptions, NumberStyle, Numeric, PluralCategory,
    PluralKind, RelativeItem, RelativeOptions, Service, Strength, TimeOptions, TimeUnit, Width,
    Word, ZoneStyle,
};
use self::bindings::nvs::ext::types::{Date, Error as WitError, TimeOfDay};
use crate::call::Error;

/// The types `bindgen!` writes for the world that exports `nvs:intl/icu` alone. Its calls are
/// made through [`crate::typed::call`], so only the types are used.
#[allow(
    missing_docs,
    unreachable_pub,
    dead_code,
    clippy::all,
    reason = "generated code, of which only the types are named"
)]
#[allow(
    ambiguous_associated_items,
    reason = "`case-first`'s case `lower` is a variant named as the `Lower` derive's associated \
              type, which is the only one of the two a type can name"
)]
mod bindings {
    wasmtime::component::bindgen!({
        path: ["../../wit/nvs-ext", "../../wit/intl.wit"],
        inline: "
            package nvs:typed;

            world intl {
                export nvs:intl/icu@1.0.0;
            }
        ",
        world: "nvs:typed/intl",
    });
}

impl From<WitError> for Error {
    fn from(err: WitError) -> Self {
        match err {
            WitError::Invalid(message) => Self::Invalid(message),
            WitError::Parse(message) => Self::Parse(message),
            WitError::Runtime(message) => Self::Runtime(message),
        }
    }
}

world! {
    error: WitError;
    collate_order(Vec<String>, String, CollateOptions) -> Vec<u64>;
    sort_keys(Vec<String>, String, CollateOptions) -> Vec<Vec<u8>>;
    format_numbers(Vec<String>, String, NumberOptions) -> Vec<String>;
    plural_categories(Vec<String>, String, PluralKind) -> Vec<PluralCategory>;
    format_date_times(Vec<LocalDateTime>, String, DateTimeOptions) -> Vec<String>;
    format_dates(Vec<Date>, String, Length) -> Vec<String>;
    format_times(Vec<TimeOfDay>, String, TimeOptions) -> Vec<String>;
    format_relative(Vec<RelativeItem>, String, RelativeOptions) -> Vec<String>;
    format_lists(Vec<Vec<String>>, String, ListOptions) -> Vec<String>;
    word_segments(Vec<String>, String) -> Vec<Vec<Word>>;
    sentence_segments(Vec<String>, String) -> Vec<Vec<String>>;
    resolve_locales(Vec<String>, Service) -> Vec<String>;
    negotiate(String, Vec<String>, String) -> String;
}

records! {
    CollateOptions { strength, case_first, numeric, ignore_punctuation }
    NumberOptions {
        style, min_fraction_digits, max_fraction_digits, grouping, currency, currency_display,
        compact_display,
    }
    LocalDateTime { year, month, day, hour, minute, second, nanos, offset_seconds, zone }
    DateTimeOptions { length, seconds, zone }
    TimeOptions { seconds }
    RelativeItem { count, unit }
    RelativeOptions { width, numeric }
    ListOptions { type_, width }
    Word { text, word_like }
}

enums! {
    Strength [Primary, Secondary, Tertiary, Quaternary, Identical]
    CaseFirst [Off, Upper, Lower]
    NumberStyle [Decimal, Percent, Currency, Compact]
    CurrencyDisplay [Symbol, Narrow, Name]
    CompactDisplay [Short, Long]
    PluralKind [Cardinal, Ordinal]
    PluralCategory [Zero, One, Two, Few, Many, Other]
    Length [Short, Medium, Long]
    ZoneStyle [None, Offset, Location, Generic]
    TimeUnit [Second, Minute, Hour, Day, Week, Month, Quarter, Year]
    Width [Wide, Short, Narrow]
    Numeric [Always, Auto]
    ListType [And, Or, Unit]
    Service [Collation, Numbers, Plurals, Dates, RelativeTime, Lists, Segmentation]
}

cores! {
    Date { year, month, day }
    TimeOfDay { hour, minute, second, nanos }
}
