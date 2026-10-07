Gives the plural category of each number, so you can choose the right form of a word.

`PluralRules::cardinal` returns one `Core\Cldr\PluralCategory` for each number, in the same order as
the list. The category tells you which form of a word goes with the number, as in "1 file" and "2
files". The locale you pass decides the rules. English has two forms: `One` and `Other`. Russian has
four: `One`, `Few`, `Many` and `Other`.

The digits a number shows count. In English, `1` is `One`, but the `decimal` value `1.0` is `Other`,
because people write "1.0 files".

`PluralRules::cardinal` throws a `RuntimeError` when a float is not finite. It throws a `LogicError`
when the locale tag is not valid.

**Good to know:** `Core\Cldr::pluralCategory` gives the same category for one number in the languages
it knows. `PluralRules::cardinal` knows every language and takes a whole list in one call. For places,
such as "1st" and "2nd", use `PluralRules::ordinal`.
