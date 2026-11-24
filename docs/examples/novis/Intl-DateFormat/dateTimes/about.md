Writes dates with their time of day the way a language and region write them, such as "Mar 1,
2026, 9:30 PM" or "01.03.2026, 21:30".

`DateFormat::dateTimes` takes a list of `Core\Time\DateTime` values and a locale tag, and returns one
string for each value. Each `Core\Time\DateTime` has its own time zone, and the
time is written as a clock in that zone shows it.

`length` is a `DateFormat::Length`, and the default is `Medium`. In English, `Short` gives "3/1/26,
9:30 PM" and `Long` gives "March 1, 2026 at 9:30 PM". Set `seconds` to `true` to show the seconds.

`zone` is a `DateFormat::ZoneStyle` and adds the time zone. For a time in Vienna, `Offset` adds the
difference from UTC, "GMT+1". `Location` adds the country, "Austria Time". `Generic` adds the name of
the zone, "Central European Time". The default is `None`, which adds nothing.

`fields` is a `DateFormat::Fields` and chooses the parts of the date. The default is
`YearMonthDay`.

`DateFormat::dateTimes` throws a `LogicError` when the locale tag is not valid. It also throws one
when `fields` has no day, such as `YearMonth`.

**Good to know:** the method does not change a value's time zone. Convert the value to the
reader's zone first.
