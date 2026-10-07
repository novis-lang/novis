//! Relative time: `format-relative` over `icu_experimental`'s `RelativeTimeFormatter` (ADR 0277
//! § 1 and § 9, `rule:core-classes/intl-batch-shape`).
//!
//! Each item is a whole count of one unit, and a negative count is in the past. ICU4X has one
//! formatter per unit and width, so a call builds at most one per unit its items name and reuses
//! it for every item of that unit. [`Numeric::Auto`] writes the locale's word where it has one
//! ("yesterday" for `-1` days, "next year" for `1` year) and the number otherwise; a count of `0`
//! is "in 0 days" under [`Numeric::Always`].

use fixed_decimal::Decimal;
use icu_experimental::relativetime::options::Numeric as IcuNumeric;
use icu_experimental::relativetime::{RelativeTimeFormatter, RelativeTimeFormatterOptions};
use icu_locale::Locale;

use crate::Error;

/// The unit a count is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// How long the unit's words are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Wide,
    Short,
    Narrow,
}

/// Whether a count the locale has a word for is written as that word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numeric {
    Always,
    Auto,
}

/// One count of one unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub count: i64,
    pub unit: Unit,
}

/// The options of `format-relative`. An absent field is the default: `Wide` and `Always`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub width: Option<Width>,
    pub numeric: Option<Numeric>,
}

fn formatter(
    locale: &Locale,
    unit: Unit,
    width: Width,
    options: RelativeTimeFormatterOptions,
) -> Result<RelativeTimeFormatter, icu_provider::DataError> {
    use RelativeTimeFormatter as F;
    let prefs = locale.into();
    match (width, unit) {
        (Width::Wide, Unit::Second) => F::try_new_long_second(prefs, options),
        (Width::Wide, Unit::Minute) => F::try_new_long_minute(prefs, options),
        (Width::Wide, Unit::Hour) => F::try_new_long_hour(prefs, options),
        (Width::Wide, Unit::Day) => F::try_new_long_day(prefs, options),
        (Width::Wide, Unit::Week) => F::try_new_long_week(prefs, options),
        (Width::Wide, Unit::Month) => F::try_new_long_month(prefs, options),
        (Width::Wide, Unit::Quarter) => F::try_new_long_quarter(prefs, options),
        (Width::Wide, Unit::Year) => F::try_new_long_year(prefs, options),
        (Width::Short, Unit::Second) => F::try_new_short_second(prefs, options),
        (Width::Short, Unit::Minute) => F::try_new_short_minute(prefs, options),
        (Width::Short, Unit::Hour) => F::try_new_short_hour(prefs, options),
        (Width::Short, Unit::Day) => F::try_new_short_day(prefs, options),
        (Width::Short, Unit::Week) => F::try_new_short_week(prefs, options),
        (Width::Short, Unit::Month) => F::try_new_short_month(prefs, options),
        (Width::Short, Unit::Quarter) => F::try_new_short_quarter(prefs, options),
        (Width::Short, Unit::Year) => F::try_new_short_year(prefs, options),
        (Width::Narrow, Unit::Second) => F::try_new_narrow_second(prefs, options),
        (Width::Narrow, Unit::Minute) => F::try_new_narrow_minute(prefs, options),
        (Width::Narrow, Unit::Hour) => F::try_new_narrow_hour(prefs, options),
        (Width::Narrow, Unit::Day) => F::try_new_narrow_day(prefs, options),
        (Width::Narrow, Unit::Week) => F::try_new_narrow_week(prefs, options),
        (Width::Narrow, Unit::Month) => F::try_new_narrow_month(prefs, options),
        (Width::Narrow, Unit::Quarter) => F::try_new_narrow_quarter(prefs, options),
        (Width::Narrow, Unit::Year) => F::try_new_narrow_year(prefs, options),
    }
}

/// Each item written in `tag`'s words, in input order.
pub fn format(items: &[Item], tag: &str, options: Options) -> Result<Vec<String>, Error> {
    let locale = crate::locale(tag)?;
    let width = options.width.unwrap_or(Width::Wide);
    let mut icu_options = RelativeTimeFormatterOptions::default();
    icu_options.numeric = match options.numeric.unwrap_or(Numeric::Always) {
        Numeric::Always => IcuNumeric::Always,
        Numeric::Auto => IcuNumeric::Auto,
    };
    let mut formatters: [Option<RelativeTimeFormatter>; 8] = Default::default();
    items
        .iter()
        .map(|item| {
            let slot = &mut formatters[item.unit as usize];
            if slot.is_none() {
                *slot = Some(formatter(&locale, item.unit, width, icu_options).map_err(|err| {
                    Error::Runtime(format!("No relative-time data for `{tag}`: {err}."))
                })?);
            }
            let formatter = slot.as_ref().expect("the slot was filled above");
            Ok(formatter.format(Decimal::from(item.count)).to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(items: &[(i64, Unit)], tag: &str, numeric: Numeric) -> Vec<String> {
        let items: Vec<Item> = items
            .iter()
            .map(|&(count, unit)| Item { count, unit })
            .collect();
        let options = Options {
            width: None,
            numeric: Some(numeric),
        };
        format(&items, tag, options).unwrap()
    }

    #[test]
    fn a_negative_count_is_in_the_past() {
        assert_eq!(
            wide(
                &[(-3, Unit::Day), (2, Unit::Hour), (-1, Unit::Year)],
                "en",
                Numeric::Always
            ),
            ["3 days ago", "in 2 hours", "1 year ago"]
        );
    }

    #[test]
    fn auto_writes_the_locale_word() {
        assert_eq!(
            wide(&[(-1, Unit::Day), (1, Unit::Day), (5, Unit::Day)], "en", Numeric::Auto),
            ["yesterday", "tomorrow", "in 5 days"]
        );
        assert_eq!(
            wide(&[(-1, Unit::Day), (-2, Unit::Day)], "de", Numeric::Auto),
            ["gestern", "vorgestern"]
        );
    }

    #[test]
    fn a_malformed_tag_is_invalid() {
        let items = [Item {
            count: 1,
            unit: Unit::Day,
        }];
        assert!(matches!(
            format(&items, "not a tag", Options::default()),
            Err(Error::Invalid(_))
        ));
    }
}
