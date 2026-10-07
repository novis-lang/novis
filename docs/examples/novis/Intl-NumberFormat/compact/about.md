Writes large numbers in a short form, the way a language and region write them.

`NumberFormat::compact` returns one string for each number, in the same order as the list. In
English, `1234` is `1.2K` and `1500000` is `1.5M`. A number under one thousand is written as it is.
Each language has its own short forms, so the locale decides the result.

The `display` option chooses the form. `CompactDisplay::Short` is the default and writes `1.2K`.
`CompactDisplay::Long` writes the word, such as `1.2 thousand`.

`NumberFormat::compact` throws a `RuntimeError` when a float is not finite. It throws a `LogicError`
when the locale tag is not valid.

**Good to know:** the short form is rounded, so it is for reading. Show the full number with
`NumberFormat::decimal` where the reader needs every digit.
