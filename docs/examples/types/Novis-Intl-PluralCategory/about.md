The plural form a number needs in a language: `Zero`, `One`, `Two`, `Few`, `Many` or `Other`.

A program uses the category to pick the right word for a number. English uses only `One` and
`Other`: "1 file", "5 files". Russian uses `One`, `Few` and `Many` for whole numbers, and 21 is
`One`, like 1. Every language has `Other`, so a program always has a word for it.

`Icu::pluralCategories` returns this enum. `PluralRules::cardinal` and `PluralRules::ordinal` return
the same cases as `Core\Cldr\PluralCategory`, which most programs use.

**Good to know:** the category is not the number. `One` in Russian also covers 21, 31 and 101.
