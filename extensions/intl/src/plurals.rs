//! Plural rules: `plural-categories` over `icu_plurals` (ADR 0277 § 4 and § 5,
//! `rule:core-classes/intl-batch-shape`).
//!
//! A number crosses as its decimal text, parsed exactly as [`crate::numbers`] parses it, so the
//! digits it shows decide its category: English puts `1` in `one` and `1.0` in `other`, as
//! `Core\Cldr` does. One rule set is built per call from the locale and the kind, and every number
//! of the batch is classified by it.
//!
//! `icu_plurals`' compiled data covers the languages at CLDR's basic coverage level and above, which
//! leaves out about sixty of the languages `Core\Cldr`'s roster carries rules for. Those rules are
//! [`CARDINAL`] and [`ORDINAL`] here, as CLDR 46's own rule strings, parsed by `icu_plurals` when a
//! call names one of the languages. What they add to the guest is mostly the parser's code, which
//! the `serde` feature of `icu_plurals` compiles in. `mo`, `sh` and `tl` are CLDR aliases the compiled data does not resolve, so they are read
//! as `ro`, `sr` and `fil`. Any other language with no rules falls back along CLDR's chain to
//! the root locale, whose only category is `other`. `crates/nvs-ext/tests/intl.rs` holds every
//! language on the roster to `Core\Cldr`'s answer.

use icu_locale::Locale;
use icu_locale::subtags::language;
use icu_plurals::provider::rules::runtime::ast::Rule;
use icu_plurals::provider::{PluralRulesData, PluralsCardinalV1, PluralsOrdinalV1};
use icu_plurals::{PluralRules, PluralRulesPreferences};
use icu_provider::prelude::*;
use serde::Deserialize;
use serde::de::value::{BorrowedStrDeserializer, Error as RuleError};

/// CLDR 46's cardinal rules for each language the compiled data has none for: the conditions for
/// `zero`, `one`, `two`, `few` and `many`, in that order, and an empty string for a category the
/// language does not use. A number no condition matches is `other`.
const CARDINAL: &[(&str, [&str; 5])] = &[
    (
        "ars",
        [
            "n = 0",
            "n = 1",
            "n = 2",
            "n % 100 = 3..10",
            "n % 100 = 11..99",
        ],
    ),
    ("asa", ONE_IS_N_1),
    ("bem", ONE_IS_N_1),
    ("bez", ONE_IS_N_1),
    ("bh", ONE_IS_N_0_1),
    ("ce", ONE_IS_N_1),
    ("cgg", ONE_IS_N_1),
    ("ckb", ONE_IS_N_1),
    ("dv", ONE_IS_N_1),
    ("ff", ONE_IS_I_0_1),
    ("fur", ONE_IS_N_1),
    ("gsw", ONE_IS_N_1),
    ("guw", ONE_IS_N_0_1),
    (
        "gv",
        [
            "",
            "v = 0 and i % 10 = 1",
            "v = 0 and i % 10 = 2",
            "v = 0 and i % 100 = 0,20,40,60,80",
            "v != 0",
        ],
    ),
    ("haw", ONE_IS_N_1),
    ("io", ["", "i = 1 and v = 0", "", "", ""]),
    ("jgo", ONE_IS_N_1),
    ("jmc", ONE_IS_N_1),
    ("kab", ONE_IS_I_0_1),
    ("kaj", ONE_IS_N_1),
    ("kcg", ONE_IS_N_1),
    ("kkj", ONE_IS_N_1),
    ("kl", ONE_IS_N_1),
    ("ksb", ONE_IS_N_1),
    (
        "kw",
        [
            "n = 0",
            "n = 1",
            "n % 100 = 2,22,42,62,82 or n % 1000 = 0 and n % 100000 = 1000..20000,40000,60000,80000 \
             or n != 0 and n % 1000000 = 100000",
            "n % 100 = 3,23,43,63,83",
            "n != 1 and n % 100 = 1,21,41,61,81",
        ],
    ),
    ("lg", ONE_IS_N_1),
    ("mas", ONE_IS_N_1),
    ("mgo", ONE_IS_N_1),
    ("nah", ONE_IS_N_1),
    ("naq", ["", "n = 1", "n = 2", "", ""]),
    ("nd", ONE_IS_N_1),
    ("nnh", ONE_IS_N_1),
    ("nr", ONE_IS_N_1),
    ("ny", ONE_IS_N_1),
    ("nyn", ONE_IS_N_1),
    ("os", ONE_IS_N_1),
    ("pap", ONE_IS_N_1),
    (
        "prg",
        [
            "n % 10 = 0 or n % 100 = 11..19 or v = 2 and f % 100 = 11..19",
            "n % 10 = 1 and n % 100 != 11 or v = 2 and f % 10 = 1 and f % 100 != 11 \
             or v != 2 and f % 10 = 1",
            "",
            "",
            "",
        ],
    ),
    ("rof", ONE_IS_N_1),
    ("rwk", ONE_IS_N_1),
    ("saq", ONE_IS_N_1),
    ("sdh", ONE_IS_N_1),
    ("seh", ONE_IS_N_1),
    ("shi", ["", "i = 0 or n = 1", "", "n = 2..10", ""]),
    ("sn", ONE_IS_N_1),
    ("ss", ONE_IS_N_1),
    ("ssy", ONE_IS_N_1),
    ("teo", ONE_IS_N_1),
    ("tig", ONE_IS_N_1),
    ("ts", ONE_IS_N_1),
    ("tzm", ["", "n = 0..1 or n = 11..99", "", "", ""]),
    ("ve", ONE_IS_N_1),
    ("vo", ONE_IS_N_1),
    ("vun", ONE_IS_N_1),
    ("wa", ONE_IS_N_0_1),
    ("wae", ONE_IS_N_1),
    ("xog", ONE_IS_N_1),
    ("yi", ["", "i = 1 and v = 0", "", "", ""]),
];

/// CLDR 46's ordinal rules for each language the compiled data has none for, in [`CARDINAL`]'s shape.
const ORDINAL: &[(&str, [&str; 5])] = &[(
    "kw",
    [
        "",
        "n = 1..4 or n % 100 = 1..4,21..24,41..44,61..64,81..84",
        "",
        "",
        "n = 5 or n % 100 = 5",
    ],
)];

const ONE_IS_N_1: [&str; 5] = ["", "n = 1", "", "", ""];
const ONE_IS_N_0_1: [&str; 5] = ["", "n = 0..1", "", "", ""];
const ONE_IS_I_0_1: [&str; 5] = ["", "i = 0,1", "", "", ""];

/// One language's rules from [`CARDINAL`] or [`ORDINAL`], served to `icu_plurals` as its data.
struct Supplement(PluralRulesData<'static>);

impl Supplement {
    /// The rules `table` carries for `locale`'s language, or `None` when it carries none.
    fn find(table: &[(&str, [&'static str; 5])], locale: &Locale) -> Option<Supplement> {
        let (_, rules) = table
            .iter()
            .find(|(language, _)| *language == locale.id.language.as_str())?;
        let rule = |text: &'static str| {
            (!text.is_empty()).then(|| {
                Rule::deserialize(BorrowedStrDeserializer::<RuleError>::new(text))
                    .unwrap_or_else(|err| panic!("the rule `{text}` does not parse: {err}"))
            })
        };
        let [zero, one, two, few, many] = rules.map(rule);
        Some(Supplement(PluralRulesData {
            zero,
            one,
            two,
            few,
            many,
        }))
    }

    fn response<M>(&self) -> Result<DataResponse<M>, DataError>
    where
        M: DynamicDataMarker<DataStruct = PluralRulesData<'static>>,
    {
        Ok(DataResponse {
            metadata: Default::default(),
            payload: DataPayload::from_owned(self.0.clone()),
        })
    }
}

impl DataProvider<PluralsCardinalV1> for Supplement {
    fn load(&self, _: DataRequest) -> Result<DataResponse<PluralsCardinalV1>, DataError> {
        self.response()
    }
}

impl DataProvider<PluralsOrdinalV1> for Supplement {
    fn load(&self, _: DataRequest) -> Result<DataResponse<PluralsOrdinalV1>, DataError> {
        self.response()
    }
}

/// `locale` with the CLDR aliases the compiled data does not resolve replaced: `mo` is `ro`, `sh`
/// is `sr` and `tl` is `fil`. `sh` is `sr-Latn` in CLDR, but CLDR's parent of `sr-Latn` is the root
/// locale, and Serbian's plural rules are the same in both scripts.
fn unalias(mut locale: Locale) -> Locale {
    match locale.id.language.as_str() {
        "mo" => locale.id.language = language!("ro"),
        "sh" => locale.id.language = language!("sr"),
        "tl" => locale.id.language = language!("fil"),
        _ => {}
    }
    locale
}

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
    let locale = unalias(crate::locale(tag)?);
    let prefs = PluralRulesPreferences::from(&locale);
    let numbers = numbers
        .iter()
        .map(|text| crate::numbers::parse(text))
        .collect::<Result<Vec<_>, _>>()?;
    let rules = match kind {
        Kind::Cardinal => match Supplement::find(CARDINAL, &locale) {
            Some(rules) => PluralRules::try_new_cardinal_unstable(&rules, prefs),
            None => PluralRules::try_new_cardinal(prefs),
        },
        Kind::Ordinal => match Supplement::find(ORDINAL, &locale) {
            Some(rules) => PluralRules::try_new_ordinal_unstable(&rules, prefs),
            None => PluralRules::try_new_ordinal(prefs),
        },
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
    fn every_carried_rule_parses() {
        for table in [CARDINAL, ORDINAL] {
            for (language, _) in table {
                let locale = crate::locale(language).unwrap();
                assert!(Supplement::find(table, &locale).is_some(), "{language}");
            }
        }
    }

    #[test]
    fn carried_rules_and_aliases_answer() {
        use Category::{Few, Many, One, Other, Two, Zero};
        assert_eq!(
            of(&["0", "1", "2", "3", "21", "1000"], "kw", Kind::Cardinal),
            [Zero, One, Two, Few, Many, Two]
        );
        assert_eq!(
            of(&["1", "5", "6", "105"], "kw", Kind::Ordinal),
            [One, Many, Other, Many]
        );
        assert_eq!(
            of(&["1", "2", "20"], "mo", Kind::Cardinal),
            [One, Few, Other]
        );
        assert_eq!(
            of(&["1", "2", "5"], "sh", Kind::Cardinal),
            [One, Few, Other]
        );
        assert_eq!(
            of(&["1", "4", "1.4"], "tl", Kind::Cardinal),
            [One, Other, Other]
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
