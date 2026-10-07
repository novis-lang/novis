Writes a list of calendar dates in the way a language and region write them.

`Icu::formatDates` takes a list of `Core\Time\Date` values, a locale tag and a `Length` case. It
returns one string for each date, in the same order. In American English, March 1, 2026 with
`Length::Long` gives "March 1, 2026".

`Icu::formatDates` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::formatDates` directly. `DateFormat::dates` calls it,
and its `length` option has a default.
