How long a date is when `DateFormat` writes it.

You pass a `Length` to `DateFormat::dates` or `DateFormat::dateTimes` as `length`. In English for the
United States, the same day is:

- `Short`: "3/1/26", only digits.
- `Medium`: "Mar 1, 2026", with a short month name. This is the default.
- `Long`: "March 1, 2026", with the full month name.

Each locale has its own patterns, so the order of day, month and year follows the locale.

**Good to know:** a `Short` date can be read in two ways by people from different countries. Use
`Medium` or `Long` when the reader's locale is not certain.
