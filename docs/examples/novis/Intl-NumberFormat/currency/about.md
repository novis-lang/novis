Writes amounts of money, the way a language and region write them.

`NumberFormat::currency` returns one string for each amount, in the same order as the list. The
second argument is the currency, as its three-letter ISO 4217 code, such as `"EUR"` or `"USD"`. The
locale decides where the symbol goes and which separators are used. In English, 1234.5 euros is
`€1,234.50`. In German, it is `1.234,50 €`.

Each amount is rounded to the digits the currency uses: 2 for euros and dollars, none for yen. The
`display` option chooses how the currency is shown. `CurrencyDisplay::Symbol` is the default and
writes `€`. `CurrencyDisplay::Narrow` writes the shortest symbol. `CurrencyDisplay::Name` writes the
name, such as `2.00 euros`.

`NumberFormat::currency` throws a `LogicError` when the currency code does not have three letters, or
when the locale tag is not valid. It throws a `RuntimeError` when a float is not finite.

**Good to know:** a float cannot store most amounts of cents exactly. Use `decimal` values for money,
and the amount you pass is the amount that is written.
