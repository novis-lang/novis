Which parts of a date `DateFormat` writes: the day, the month, the year and the day of the week.

`DateFormat::Fields` is declared in the class `Novis\Intl\DateFormat`.

You pass a `DateFormat::Fields` to `DateFormat::dates` or `DateFormat::dateTimes` as `fields`, beside
`length`. Each case of the enum is one choice. In English for the United States, with
`DateFormat::Length::Long`, Monday, January 5, 2026 is:

- `Day`: "5".
- `MonthDay`: "January 5".
- `YearMonthDay`: "January 5, 2026". This is the default.
- `Weekday`: "Monday".
- `DayWeekday`: "5 Monday".
- `MonthDayWeekday`: "Monday, January 5".
- `YearMonthDayWeekday`: "Monday, January 5, 2026".
- `Month`: "January".
- `YearMonth`: "January 2026".
- `Year`: "2026".

The name of a case says which parts appear. The locale decides in which order they appear. In German,
`YearMonth` gives "Januar 2026" and `MonthDayWeekday` gives "Montag, 5. Januar". The locale also
decides the separators. In English, `DayWeekday` has no comma and gives "5 Monday".

`Month`, `YearMonth` and `Year` have no day. A time of day needs a day, so `DateFormat::dateTimes`
throws a `LogicError` for these three cases. It throws before it formats any value. Use
`DateFormat::dates` for them.

**Good to know:** `length` still applies. With the default `DateFormat::Length::Medium`, `YearMonth`
gives "Jan 2026" in English.
