Writes numbers as percentages, the way a language and region write them.

`NumberFormat::percent` returns one string for each number, in the same order as the list. The number
is multiplied by 100, so `0.25` is `25%` and `1` is `100%`. The locale decides how the result looks.
English writes `25%`. German writes `25 %`, with a space before the sign.

By default the result has no fraction digits, so `0.125` is `13%`. The options are the same as for
`NumberFormat::decimal`: `minFractionDigits`, `maxFractionDigits` and `grouping`.

`NumberFormat::percent` throws a `RuntimeError` when a float is not finite. It throws a `LogicError`
when the locale tag or an option is not valid.

**Good to know:** pass the share, not the percentage. For 25 percent, pass `0.25`, not `25`.
