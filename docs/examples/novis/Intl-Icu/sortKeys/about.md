Returns a sort key for each string in a list, in the order of the locale.

`Icu::sortKeys` takes a list of strings, a locale tag and a shape of sort options. It returns one
`bytes` value for each string, in the same order. Two keys compare as bytes in the same order as the
two strings compare in that locale. You can store a key in a database column and sort the rows by it.

`Icu::sortKeys` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::sortKeys` directly. `Collator::sortKeys` is the
same function with options that are easier to write.
