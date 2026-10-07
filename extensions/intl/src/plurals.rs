//! Plural rules: `plural-categories` over `icu_plurals` (ADR 0277 § 4 and § 5,
//! `rule:core-classes/intl-batch-shape`).
//!
//! A number crosses as its decimal text, parsed exactly as [`crate::numbers`] parses it, so the
//! digits it shows decide its category: English puts `1` in `one` and `1.0` in `other`, as
//! `Core\Cldr` does. One rule set is built per call from the locale and the kind, and every number
//! of the batch is classified by it. A language with no rules of its own falls back along CLDR's
//! chain to the root locale, whose only category is `other`.

use icu_plurals::{PluralRules, PluralRulesPreferences};

pub use icu_plurals::PluralCategory as Category;

use crate::Error;

/// The WIT `plural-kind`: the forms a count takes, or the forms a place takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cardinal,
    Ordinal,
}

/// The category of each of `numbers`, given as its decimal text, in `locale` under `kind`.
pub fn categories(numbers: &[String], locale: &str, kind: Kind) -> Result<Vec<Category>, Error> {
    let tag = locale;
    let prefs = PluralRulesPreferences::from(&crate::locale(tag)?);
    let numbers = numbers
        .iter()
        .map(|text| crate::numbers::parse(text))
        .collect::<Result<Vec<_>, _>>()?;
    let rules = match kind {
        Kind::Cardinal => PluralRules::try_new_cardinal(prefs),
        Kind::Ordinal => PluralRules::try_new_ordinal(prefs),
    }
    .map_err(|err| Error::Runtime(format!("No plural rules for `{tag}`: {err}.")))?;
    Ok(numbers
        .iter()
        .map(|number| rules.category_for(number))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(list: &[&str], locale: &str, kind: Kind) -> Vec<Category> {
        let input: Vec<String> = list.iter().map(|s| (*s).to_owned()).collect();
        categories(&input, locale, kind).unwrap()
    }

    #[test]
    fn english_counts_the_digits_a_number_shows() {
        use Category::{One, Other};
        assert_eq!(
            of(&["1", "1.0", "2", "0"], "en", Kind::Cardinal),
            [One, Other, Other, Other]
        );
    }

    #[test]
    fn russian_and_arabic_use_their_own_forms() {
        use Category::{Few, Many, One, Two, Zero};
        assert_eq!(
            of(&["1", "2", "5", "21"], "ru", Kind::Cardinal),
            [One, Few, Many, One]
        );
        assert_eq!(of(&["0", "2"], "ar", Kind::Cardinal), [Zero, Two]);
    }

    #[test]
    fn english_ordinals_are_first_second_third_and_the_rest() {
        use Category::{Few, One, Other, Two};
        assert_eq!(
            of(&["1", "2", "3", "4", "11", "22"], "en", Kind::Ordinal),
            [One, Two, Few, Other, Other, Two]
        );
    }

    #[test]
    fn text_that_is_not_a_number_is_invalid() {
        let err = categories(&["one".to_owned()], "en", Kind::Cardinal).unwrap_err();
        assert!(
            matches!(err, Error::Invalid(ref m) if m.contains("one")),
            "{err:?}"
        );
    }
}
