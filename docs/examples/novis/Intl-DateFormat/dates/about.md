Writes calendar dates the way a language and region write them, such as "Dec 24, 2026" or
"24.12.2026".

`DateFormat::dates` takes a list of `Core\Time\Date` values and a locale tag, and returns one string
for each date, in the same order. The locale decides the order of day, month and year, the month
names and the separators. A locale tag is a language and region, such as `en-GB` or `ja`. With
`en-GB`, December 24, 2026 gives "24 Dec 2026". With `ja`, the same date gives "2026/12/24".

The `length` option is `DateFormat::Length::Short`, `DateFormat::Length::Medium` or `DateFormat::Length::Long`, and the default is
`DateFormat::Length::Medium`. In German, March 1, 2026 gives "01.03.26" with `Short`, "01.03.2026" with `Medium` and "1. März 2026"
with `Long`.

The `fields` option chooses which parts of the date are written. It is a `DateFormat::Fields`, and the
default is `DateFormat::Fields::YearMonthDay`. With `DateFormat::Length::Long` in English,
`DateFormat::Fields::YearMonth` gives "December 2026". `DateFormat::Fields::MonthDayWeekday` gives
"Thursday, December 24". The locale decides the order of the parts.

`DateFormat::dates` throws a `LogicError` when the locale tag is not valid.

**Good to know:** a `Core\Time\Date` has no time of day and no time zone. To write the date of a
`Core\Time\DateTime`, call `$value->date()` first.
