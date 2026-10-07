Returns the sorted order of a list of strings, as positions in the list.

`Icu::collateOrder` takes a list of strings, a locale tag and a shape of sort options. It returns one
number for each string. The first number is the position of the string that comes first, and so on.
For `["b", "a", "ä"]` in German, the result is `[1, 2, 0]`. The options shape has the keys
`strength`, `caseFirst`, `numeric` and `ignorePunctuation`.

`Icu::collateOrder` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::collateOrder` directly. `Collator::sort` calls it
and returns the strings in sorted order.
