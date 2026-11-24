Reads a date from text. You give a pattern that says how the text is written, such as `dd.MM.yyyy`.
`Date::parse('05.01.2026', 'dd.MM.yyyy')` returns the date `2026-01-05`. The pattern uses CLDR
letters, the standard pattern letters that `format` also uses. `dd` is the day, `MM` is the month
and `yyyy` is the year.

The pattern may contain date letters only. A time letter such as `HH` is not allowed. A zone letter
such as `XXX`, for the time zone, is not allowed either. A `Date` has no time and no zone. When the
pattern is a string literal, the program does not compile. When the pattern is a variable, `parse`
throws a `LogicError`.

When the text does not match the pattern, `parse` throws a `ParseError`. This happens when a number
is missing, when there is extra text at the end, or when the date does not exist, such as
`30.02.2026`.

When the pattern has no day, the day is `1`. When it has no month, the month is `1`.
`Date::parse('03/2026', 'MM/yyyy')` returns `2026-03-01`.

**Good to know:** `Date::fromIso` reads the format `yyyy-MM-dd` with no pattern. `format` writes a
date with the same pattern letters.
