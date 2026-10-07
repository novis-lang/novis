//! Numbers: `format-numbers` over `icu_decimal`'s decimal and compact formatters and
//! `icu_experimental`'s percent and currency formatters (ADR 0277 § 4,
//! `rule:core-classes/intl-batch-shape`).
//!
//! A number crosses as its decimal text, which [`fixed_decimal`] parses exactly, so a `decimal` is
//! never rounded through `f64`; an exponent is accepted with or without its `+`, the way Novis
//! writes a large `float`. One formatter is built per call from the locale and the options, and
//! every number of the batch is written by it.
//!
//! The four styles take different options, as the four `NumberFormat` members do. `decimal` and
//! `percent` take the fraction digits and the grouping; absent, the locale's defaults apply, which
//! are 0 to 3 fraction digits for `decimal` and none for `percent`. A number is rounded half away
//! from zero to the largest count, then written with at least the smallest one, and a value that
//! rounds to zero loses its minus sign. `percent` multiplies by 100, so `0.25` is 25 percent.
//! `currency` needs an ISO 4217 code and rounds to that currency's own digits, and `compact`
//! writes `1.2K` or `1.2 thousand`. An option a style does not take is `Invalid`, as is a
//! fraction count over [`MAX_FRACTION_DIGITS`].

use fixed_decimal::{Decimal, Sign, SignedRoundingMode, UnsignedRoundingMode};
use icu_decimal::options::{
    CompactDecimalFormatterOptions, DecimalFormatterOptions, GroupingStrategy,
};
use icu_decimal::{CompactDecimalFormatter, DecimalFormatter};
use icu_experimental::dimension::currency::CurrencyType;
use icu_experimental::dimension::currency::formatter::CurrencyFormatter;
use icu_experimental::dimension::percent::formatter::PercentFormatter;
use icu_locale::Locale;

use crate::Error;

/// The most fraction digits an option may ask for.
pub const MAX_FRACTION_DIGITS: u64 = 20;

/// The WIT `number-style`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Decimal,
    Percent,
    Currency,
    Compact,
}

/// The WIT `currency-display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyDisplay {
    Symbol,
    Narrow,
    Name,
}

/// The WIT `compact-display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactDisplay {
    Short,
    Long,
}

/// The WIT `number-options`: an absent field is the locale's or the currency's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub style: Style,
    pub min_fraction_digits: Option<u64>,
    pub max_fraction_digits: Option<u64>,
    pub grouping: Option<bool>,
    pub currency: Option<String>,
    pub currency_display: Option<CurrencyDisplay>,
    pub compact_display: Option<CompactDisplay>,
}

impl Options {
    /// `style` with every other field absent.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            min_fraction_digits: None,
            max_fraction_digits: None,
            grouping: None,
            currency: None,
            currency_display: None,
            compact_display: None,
        }
    }
}

/// Each of `numbers`, given as its decimal text, written in `locale` in the style of `options`.
pub fn format(numbers: &[String], locale: &str, options: &Options) -> Result<Vec<String>, Error> {
    let tag = locale;
    let locale = crate::locale(tag)?;
    check_options(options)?;
    let numbers = numbers
        .iter()
        .map(|text| parse(text))
        .collect::<Result<Vec<_>, _>>()?;
    let no_data = |err| Error::Runtime(format!("No number data for `{tag}`: {err}."));
    match options.style {
        Style::Decimal => {
            let formatter = decimal_formatter(&locale, options).map_err(no_data)?;
            let (min, max) = fraction_digits(options, 3)?;
            Ok(numbers
                .into_iter()
                .map(|number| formatter.format(&digits(number, min, max)).to_string())
                .collect())
        }
        Style::Percent => {
            let formatter = PercentFormatter::try_new_with_decimal_formatter(
                (&locale).into(),
                decimal_formatter(&locale, options).map_err(no_data)?,
                Default::default(),
            )
            .map_err(no_data)?;
            let (min, max) = fraction_digits(options, 0)?;
            Ok(numbers
                .into_iter()
                .map(|mut number| {
                    number.absolute.multiply_pow10(2);
                    number.absolute.trim_start();
                    formatter.format(&digits(number, min, max)).to_string()
                })
                .collect())
        }
        Style::Currency => currency(numbers, tag, &locale, options),
        Style::Compact => {
            let prefs = (&locale).into();
            let compact = CompactDecimalFormatterOptions::default();
            let formatter = match options.compact_display.unwrap_or(CompactDisplay::Short) {
                CompactDisplay::Short => CompactDecimalFormatter::try_new_short(prefs, compact),
                CompactDisplay::Long => CompactDecimalFormatter::try_new_long(prefs, compact),
            }
            .map_err(no_data)?;
            Ok(numbers
                .into_iter()
                .map(|number| formatter.format(&no_negative_zero(number)).to_string())
                .collect())
        }
    }
}

/// The batch in `options.currency`, rounded to that currency's digits.
fn currency(
    numbers: Vec<Decimal>,
    tag: &str,
    locale: &Locale,
    options: &Options,
) -> Result<Vec<String>, Error> {
    let code = options.currency.as_deref().unwrap_or_default();
    let currency = Some(code)
        .filter(|code| code.len() == 3 && code.bytes().all(|b| b.is_ascii_alphabetic()))
        .and_then(|code| CurrencyType::try_from_str(&code.to_ascii_lowercase()).ok())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "`{code}` is not a currency code. A currency code has three letters, such as `EUR`."
            ))
        })?;
    let prefs = locale.into();
    let formatter = match options.currency_display.unwrap_or(CurrencyDisplay::Symbol) {
        CurrencyDisplay::Symbol => {
            CurrencyFormatter::try_new_symbol(prefs, currency, Default::default())
        }
        CurrencyDisplay::Narrow => {
            CurrencyFormatter::try_new_symbol_narrow(prefs, currency, Default::default())
        }
        CurrencyDisplay::Name => CurrencyFormatter::try_new_name(prefs, currency),
    }
    .map_err(|err| Error::Runtime(format!("No currency data for `{code}` in `{tag}`: {err}.")))?;
    Ok(numbers
        .into_iter()
        .map(|number| {
            formatter
                .format_fixed_decimal(&no_negative_zero(number))
                .to_string()
        })
        .collect())
}

/// `Invalid` for an option `options.style` does not take, or a fraction count out of range.
fn check_options(options: &Options) -> Result<(), Error> {
    let style = match options.style {
        Style::Decimal => "decimal",
        Style::Percent => "percent",
        Style::Currency => "currency",
        Style::Compact => "compact",
    };
    let digit_style = matches!(options.style, Style::Decimal | Style::Percent);
    let given = [
        (
            options.min_fraction_digits.is_some(),
            "minFractionDigits",
            digit_style,
        ),
        (
            options.max_fraction_digits.is_some(),
            "maxFractionDigits",
            digit_style,
        ),
        (options.grouping.is_some(), "grouping", digit_style),
        (
            options.currency.is_some() || options.style == Style::Currency,
            "currency",
            options.style == Style::Currency,
        ),
        (
            options.currency_display.is_some(),
            "currencyDisplay",
            options.style == Style::Currency,
        ),
        (
            options.compact_display.is_some(),
            "compactDisplay",
            options.style == Style::Compact,
        ),
    ];
    for (present, option, allowed) in given {
        if present && !allowed {
            return Err(Error::Invalid(format!(
                "The `{style}` style does not take the option `{option}`."
            )));
        }
    }
    for (digits, option) in [
        (options.min_fraction_digits, "minFractionDigits"),
        (options.max_fraction_digits, "maxFractionDigits"),
    ] {
        if digits.is_some_and(|digits| digits > MAX_FRACTION_DIGITS) {
            return Err(Error::Invalid(format!(
                "`{option}` is at most {MAX_FRACTION_DIGITS}."
            )));
        }
    }
    Ok(())
}

/// The smallest and largest count of fraction digits, from the options or `default_max`.
fn fraction_digits(options: &Options, default_max: u64) -> Result<(i16, i16), Error> {
    let min = options.min_fraction_digits.unwrap_or(0);
    let max = options
        .max_fraction_digits
        .unwrap_or_else(|| default_max.max(min));
    if min > max {
        return Err(Error::Invalid(format!(
            "`minFractionDigits` is {min}, which is more than `maxFractionDigits`, {max}."
        )));
    }
    // Both are at most `MAX_FRACTION_DIGITS`, which `check_options` has checked.
    Ok((min as i16, max as i16))
}

/// The decimal formatter for `locale` with the grouping `options` asks for.
fn decimal_formatter(
    locale: &Locale,
    options: &Options,
) -> Result<DecimalFormatter, icu_provider::DataError> {
    let mut decimal = DecimalFormatterOptions::default();
    decimal.grouping_strategy = options.grouping.map(|grouping| {
        if grouping {
            GroupingStrategy::Always
        } else {
            GroupingStrategy::Never
        }
    });
    DecimalFormatter::try_new(locale.into(), decimal)
}

/// `text` as a decimal, or `Invalid` naming it.
pub(crate) fn parse(text: &str) -> Result<Decimal, Error> {
    let exact = text.replacen("e+", "e", 1).replacen("E+", "E", 1);
    Decimal::try_from_str(&exact)
        .map_err(|_| Error::Invalid(format!("`{text}` is not a decimal number.")))
}

/// `number` rounded half away from zero to `max` fraction digits, with at least `min` of them.
fn digits(number: Decimal, min: i16, max: i16) -> Decimal {
    let mut number = number.rounded_with_mode(
        -max,
        SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand),
    );
    number.absolute.trim_end();
    number.absolute.pad_end(-min);
    no_negative_zero(number)
}

/// `number`, with no minus sign when it is zero.
fn no_negative_zero(mut number: Decimal) -> Decimal {
    if number.absolute.is_zero() {
        number.sign = Sign::None;
    }
    number
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn formatted(list: &[&str], locale: &str, options: &Options) -> Vec<String> {
        format(&strings(list), locale, options).unwrap()
    }

    #[test]
    fn decimal_groups_and_rounds_by_locale() {
        let options = Options::new(Style::Decimal);
        assert_eq!(
            formatted(&["1234567.891", "-0.0001", "19.90"], "en", &options),
            ["1,234,567.891", "0", "19.9"]
        );
        assert_eq!(
            formatted(&["1234567.891"], "de", &options),
            ["1.234.567,891"]
        );
    }

    #[test]
    fn fraction_digits_and_grouping_apply() {
        let options = Options {
            min_fraction_digits: Some(2),
            max_fraction_digits: Some(2),
            grouping: Some(false),
            ..Options::new(Style::Decimal)
        };
        assert_eq!(
            formatted(&["1234.5", "2.345", "1.0E+3"], "en", &options),
            ["1234.50", "2.35", "1000.00"]
        );
    }

    #[test]
    fn percent_multiplies_by_one_hundred() {
        let options = Options::new(Style::Percent);
        assert_eq!(
            formatted(&["0.25", "1", "-0.123"], "en", &options),
            ["25%", "100%", "-12%"]
        );
        assert_eq!(formatted(&["0.25"], "de", &options), ["25\u{a0}%"]);
    }

    #[test]
    fn currency_rounds_to_its_own_digits() {
        let options = Options {
            currency: Some("EUR".to_owned()),
            ..Options::new(Style::Currency)
        };
        assert_eq!(
            formatted(&["1234.5", "3"], "de", &options),
            ["1.234,50\u{a0}€", "3,00\u{a0}€"]
        );
        let yen = Options {
            currency: Some("JPY".to_owned()),
            ..Options::new(Style::Currency)
        };
        assert_eq!(formatted(&["1234.5"], "en", &yen), ["¥1,235"]);
    }

    #[test]
    fn compact_writes_short_and_long() {
        let short = Options::new(Style::Compact);
        assert_eq!(
            formatted(&["1234", "1500000", "999"], "en", &short),
            ["1.2K", "1.5M", "999"]
        );
        let long = Options {
            compact_display: Some(CompactDisplay::Long),
            ..short
        };
        assert_eq!(formatted(&["1234"], "en", &long), ["1.2 thousand"]);
    }

    #[test]
    fn an_option_the_style_does_not_take_is_invalid() {
        let options = Options {
            grouping: Some(true),
            ..Options::new(Style::Compact)
        };
        let err = format(&strings(&["1"]), "en", &options).unwrap_err();
        assert!(
            matches!(err, Error::Invalid(ref m) if m.contains("grouping")),
            "{err:?}"
        );
        let err = format(&strings(&["1"]), "en", &Options::new(Style::Currency)).unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    }

    #[test]
    fn text_that_is_not_a_number_is_invalid() {
        let err = format(&strings(&["NaN"]), "en", &Options::new(Style::Decimal)).unwrap_err();
        assert!(
            matches!(err, Error::Invalid(ref m) if m.contains("NaN")),
            "{err:?}"
        );
    }
}
