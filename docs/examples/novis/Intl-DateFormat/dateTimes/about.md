Writes dates with their time of day the way a language and region write them, such as "Mar 1,
2026, 9:30 PM" or "01.03.2026, 21:30".

`DateFormat::dateTimes` takes a list of `Core\Time\DateTime` values and a locale tag, and returns one
string for each value, in the same order. Each value is written as the clock shows it in its own time
zone. Two values for the same moment in Vienna and in New York give two different times of day.

The options change what is shown. `length` is `Length::Short`, `Length::Medium` or `Length::Long`,
and the default is `Length::Medium`. In English, `Length::Short` gives "3/1/26, 9:30 PM" and
`Length::Long` gives "March 1, 2026 at 9:30 PM". Set `seconds` to `true` to show the seconds. `zone`
adds the time zone: `ZoneStyle::Offset` adds "GMT+1", `ZoneStyle::Location` adds "Austria Time", and
`ZoneStyle::Generic` adds "Central European Time". The default is `ZoneStyle::None`, which adds
nothing.

`DateFormat::dateTimes` throws a `LogicError` when the locale tag is not valid.

**Good to know:** the method does not change a value's time zone. To show a time in the reader's
zone, convert the value first, for example with `$value->toInstant()->in($zone)`.
