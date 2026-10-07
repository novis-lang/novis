How `NumberFormat::currency` writes the currency of an amount.

You pass a `CurrencyDisplay` as `display`:

- `Symbol` writes the symbol that is clear in the locale, such as "US$" in Canada. This is the
  default.
- `Narrow` writes the shortest symbol, such as "$". It is shorter, but a reader may not know which
  dollar it is.
- `Name` writes the name of the currency, such as "US dollars".

The name follows the plural rules of the locale, so the amount and the word always match.

**Good to know:** use `Symbol` when one page can show prices in more than one currency.
