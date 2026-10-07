Writes calendar dates the way a language and region write them, such as "Dec 24, 2026" or
"24.12.2026".

`DateFormat::dates` takes a list of `Core\Time\Date` values and a locale tag, and returns one string
for each date, in the same order. The locale decides the order of day, month and year, the month
names and the separators. In British English, December 24, 2026 gives "24 Dec 2026". In Japanese, it
gives "2026/12/24".

The `length` option is `Length::Short`, `Length::Medium` or `Length::Long`, and the default is
`Length::Medium`. In German, March 1, 2026 gives "01.03.26", "01.03.2026" and "1. März 2026".

`DateFormat::dates` throws a `LogicError` when the locale tag is not valid.

**Good to know:** a `Core\Time\Date` has no time of day and no time zone. To write the date of a
`Core\Time\DateTime`, call `$value->date()` first.
