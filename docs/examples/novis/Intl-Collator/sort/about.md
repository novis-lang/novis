Sorts a list of strings by the rules of a language and region.

`Collator::sort` returns the same strings in a new order. The order follows the locale you pass, such
as `"de"` or `"sv-SE"`. In German, `ä` sorts next to `a`. In Swedish, `ä` sorts after `z`. Equal
strings keep the order they had in the list.

The options change how strings compare. `strength` sets which differences count: with
`Strength::Primary`, case and accents do not count. `caseFirst` puts upper case or lower case first.
`numeric` sorts digits by their value, so `"9"` comes before `"10"`. `ignorePunctuation` skips spaces
and punctuation.

`Collator::sort` throws a `LogicError` when the locale tag is not valid. A valid tag with no data of
its own uses the general rules.

**Good to know:** the whole list is sorted in one call. To sort rows by a column, or to store the
order in a database, use `Collator::sortKeys`.
