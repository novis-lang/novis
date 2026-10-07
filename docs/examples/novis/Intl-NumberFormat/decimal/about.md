Writes numbers with the separators of a language and region.

`NumberFormat::decimal` returns one string for each number, in the same order as the list. The locale
you pass, such as `"en"` or `"de"`, decides the separators. English writes `1,234.5`. German writes
`1.234,5`.

By default the result has up to 3 fraction digits, and the digits are grouped. The options change
this. `minFractionDigits` and `maxFractionDigits` set how many fraction digits the result has, from 0
to 20. `grouping: false` turns the group separators off. A number is rounded half away from zero.

The list may mix `int`, `float` and `decimal` values. A `decimal` keeps every digit it has.

`NumberFormat::decimal` throws a `RuntimeError` when a float is not finite, such as
`Core\Math::INFINITY`. It throws a `LogicError` when the locale tag or an option is not valid.

**Good to know:** the whole list is formatted in one call. To format one number, pass a list with one
number.
