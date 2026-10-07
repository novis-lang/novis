//! Collation: `collate-order` and `sort-keys` over `icu_collator` (ADR 0277 § 3,
//! `rule:core-classes/intl-batch-shape`).
//!
//! One collator is built per call from the locale and the options, and every string of the batch
//! is ordered or keyed by it. `order` returns, for each place in the locale's order, the index of
//! the string that goes there, so only numbers cross back; the sort is stable, so equal strings
//! keep their input order. A sort key is the collator's own (`CollatorBorrowed::write_sort_key_to`):
//! two keys compare by byte order exactly as the collator orders their strings, and a key is valid
//! only under the ICU4X version this crate pins.
//!
//! An option given both in the options and as the locale's own keyword — `strength` beside
//! `-u-ks-`, `caseFirst` beside `-u-kf-`, `numeric` beside `-u-kn-` — is `Invalid`. A collation
//! type the locale names, such as `de-u-co-phonebk`, comes from the tag.

use icu_collator::options::{AlternateHandling, CollatorOptions};
use icu_collator::preferences::{CollationCaseFirst, CollationNumericOrdering};
use icu_collator::{CollatorBorrowed, CollatorPreferences};
use icu_locale::Locale;
use icu_locale::extensions::unicode::{Key, key};

pub use icu_collator::options::Strength;

use crate::Error;

/// Where uppercase sorts against lowercase, the WIT `case-first` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseFirst {
    Off,
    Upper,
    Lower,
}

/// The WIT `collate-options`: an absent field is the collator's default, `Tertiary`, `Off`, and
/// `false` for the two flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub strength: Option<Strength>,
    pub case_first: Option<CaseFirst>,
    pub numeric: Option<bool>,
    pub ignore_punctuation: Option<bool>,
}

/// For each place in `locale`'s order, the index in `strings` of the string that goes there.
pub fn order(strings: &[String], locale: &str, options: &Options) -> Result<Vec<u64>, Error> {
    let collator = collator(locale, options)?;
    let mut order: Vec<usize> = (0..strings.len()).collect();
    order.sort_by(|&a, &b| collator.compare(&strings[a], &strings[b]));
    Ok(order.into_iter().map(|index| index as u64).collect())
}

/// One sort key per string, in the order of `strings`.
pub fn sort_keys(
    strings: &[String],
    locale: &str,
    options: &Options,
) -> Result<Vec<Vec<u8>>, Error> {
    let collator = collator(locale, options)?;
    Ok(strings
        .iter()
        .map(|string| {
            let mut key = Vec::new();
            let Ok(()) = collator.write_sort_key_to(string, &mut key);
            key
        })
        .collect())
}

/// The collator for `locale` under `options`.
fn collator(locale: &str, options: &Options) -> Result<CollatorBorrowed<'static>, Error> {
    let tag = locale;
    let locale = crate::locale(tag)?;
    let mut prefs = CollatorPreferences::from(&locale);
    let mut icu = CollatorOptions::default();
    if let Some(strength) = options.strength {
        no_keyword(&locale, tag, key!("ks"), "strength")?;
        icu.strength = Some(strength);
    }
    if let Some(case_first) = options.case_first {
        no_keyword(&locale, tag, key!("kf"), "caseFirst")?;
        prefs.case_first = Some(match case_first {
            CaseFirst::Off => CollationCaseFirst::False,
            CaseFirst::Upper => CollationCaseFirst::Upper,
            CaseFirst::Lower => CollationCaseFirst::Lower,
        });
    }
    if let Some(numeric) = options.numeric {
        no_keyword(&locale, tag, key!("kn"), "numeric")?;
        prefs.numeric_ordering = Some(if numeric {
            CollationNumericOrdering::True
        } else {
            CollationNumericOrdering::False
        });
    }
    if options.ignore_punctuation == Some(true) {
        icu.alternate_handling = Some(AlternateHandling::Shifted);
    }
    CollatorBorrowed::try_new(prefs, icu)
        .map_err(|err| Error::Runtime(format!("No collation data for `{tag}`: {err}.")))
}

/// `Invalid` when `locale` carries the Unicode keyword `keyword`, which sets what `option` sets.
fn no_keyword(locale: &Locale, tag: &str, keyword: Key, option: &str) -> Result<(), Error> {
    if locale.extensions.unicode.keywords.get(&keyword).is_some() {
        return Err(Error::Invalid(format!(
            "The locale `{tag}` sets `-u-{keyword}-`, and the options set `{option}` too. Set it in one place only."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn sorted(list: &[&str], locale: &str) -> Vec<String> {
        let input = strings(list);
        order(&input, locale, &Options::default())
            .unwrap()
            .into_iter()
            .map(|index| input[index as usize].clone())
            .collect()
    }

    #[test]
    fn swedish_puts_a_umlaut_after_z_and_german_does_not() {
        assert_eq!(sorted(&["ä", "z", "a"], "sv"), ["a", "z", "ä"]);
        assert_eq!(sorted(&["ä", "z", "a"], "de"), ["a", "ä", "z"]);
    }

    #[test]
    fn keys_order_as_the_collator_orders() {
        let input = strings(&["Zebra", "apple", "Äpfel", "zebra", "Apple", "10", "9"]);
        let keys = sort_keys(&input, "de", &Options::default()).unwrap();
        let mut by_key: Vec<usize> = (0..input.len()).collect();
        by_key.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
        let by_order: Vec<usize> = order(&input, "de", &Options::default())
            .unwrap()
            .into_iter()
            .map(|index| index as usize)
            .collect();
        assert_eq!(by_key, by_order);
    }

    #[test]
    fn equal_strings_keep_their_input_order() {
        let options = Options {
            strength: Some(Strength::Primary),
            ..Options::default()
        };
        let input = strings(&["b", "A", "a", "B"]);
        assert_eq!(order(&input, "en", &options).unwrap(), [1, 2, 0, 3]);
    }

    #[test]
    fn numeric_sorts_digits_by_value() {
        let options = Options {
            numeric: Some(true),
            ..Options::default()
        };
        let input = strings(&["10", "9"]);
        assert_eq!(order(&input, "en", &options).unwrap(), [1, 0]);
        assert_eq!(order(&input, "en", &Options::default()).unwrap(), [0, 1]);
    }

    #[test]
    fn a_keyword_beside_its_option_is_invalid() {
        let options = Options {
            numeric: Some(true),
            ..Options::default()
        };
        let err = order(&strings(&["a"]), "en-u-kn", &options).unwrap_err();
        assert!(matches!(err, Error::Invalid(_)), "{err:?}");
    }

    #[test]
    fn a_malformed_tag_is_invalid() {
        let err = order(&strings(&["a"]), "en_US!", &Options::default()).unwrap_err();
        assert!(
            matches!(err, Error::Invalid(ref m) if m.contains("en_US!")),
            "{err:?}"
        );
    }
}
