The style `Icu::formatNumbers` uses to write a number: as a plain number, a percent, an amount of
money or a short count.

- `Decimal` writes the number with the digit groups and decimal sign of the locale.
- `Percent` multiplies by 100 and adds the percent sign. 0.25 is "25%".
- `Currency` writes an amount of money. It needs a currency code, such as "EUR".
- `Compact` writes a short form, such as "1.2K".

Most programs do not use this enum. The four `NumberFormat` methods `decimal`, `percent`,
`currency` and `compact` each use one style, and their options are simpler. `NumberStyle` is for a
program that calls `Icu::formatNumbers` directly.
