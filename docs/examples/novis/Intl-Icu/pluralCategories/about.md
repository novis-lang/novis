Returns the plural category of each number in a list, for a language.

`Icu::pluralCategories` takes a list of numbers written as decimal text, a locale tag and a
`PluralKind`. `PluralKind::Cardinal` gives the category for a count, as in "5 files".
`PluralKind::Ordinal` gives the category for a place, as in "5th". The result has one category for
each number, in the same order.

`Icu::pluralCategories` throws a `LogicError` when the locale tag or a number is not valid.

**Good to know:** most programs do not call `Icu::pluralCategories` directly. `PluralRules::cardinal`
and `PluralRules::ordinal` call it, take `int`, `float` and `decimal` values, and return
`Core\Cldr\PluralCategory` cases.
