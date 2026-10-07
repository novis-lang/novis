Gives the category of each number as a place, such as first, second or third.

`PluralRules::ordinal` returns one `Core\Cldr\PluralCategory` for each number, in the same order as
the list. The category tells you which ending goes with the number. In English, `1` is `One` for
"1st", `2` is `Two` for "2nd", `3` is `Few` for "3rd", and `4` is `Other` for "4th". `11` is `Other`,
for "11th", and `22` is `Two`, for "22nd". The locale you pass decides the rules.

`PluralRules::ordinal` throws a `RuntimeError` when a float is not finite. It throws a `LogicError`
when the locale tag is not valid.

**Good to know:** the category is not the text. You choose the ending for each category yourself,
and each language has its own endings. To count things, as in "2 files", use
`PluralRules::cardinal`.
