How `NumberFormat::currency` writes the currency of an amount.

`NumberFormat::CurrencyDisplay` is declared in the class `Novis\Intl\NumberFormat`.

You pass a `NumberFormat::CurrencyDisplay` as `display`:

- `Symbol` writes the usual symbol of the language and region. This is the default. In Canada, "$"
  means Canadian dollars. So in English for Canada, 19.90 US dollars are written "US$19.90".
- `Narrow` writes the shortest symbol, such as "$19.90". A reader may not know which dollar it is.
- `Name` writes the name of the currency, such as "19.90 US dollars".

The name follows the plural rules of the locale, so the amount and the word always match.

**Good to know:** use `Symbol` when one page can show prices in more than one currency.
