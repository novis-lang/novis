Writes a list of numbers the way a language and region write them.

`Icu::formatNumbers` takes a list of numbers written as decimal text, such as `"1234.5"`, a locale
tag and a shape of options. It returns one string for each number, in the same order. The `style` key
is a `NumberStyle` case: `Decimal`, `Percent`, `Currency` or `Compact`. The shape names every key, and
a key that does not apply to the style is `null`. In German, `"1234.5"` with `NumberStyle::Decimal`
gives "1.234,5".

`Icu::formatNumbers` throws a `LogicError` when the locale tag or a number is not valid.

**Good to know:** most programs do not call `Icu::formatNumbers` directly. The four methods of
`NumberFormat` call it, and they take `int`, `float` and `decimal` values.
