Writes times of day the way a language and region write them, such as "9:30 PM" or "21:30".

`DateFormat::times` takes a list of `Core\Time\TimeOfDay` values and a locale tag, and returns one
string for each time, in the same order. The locale decides between a 12-hour and a 24-hour clock.
In American English, half past nine in the evening gives "9:30 PM". In German, it gives "21:30".

The result shows hours and minutes. Set the `seconds` option to `true` to show the seconds too, as
in "9:30:15 PM".

`DateFormat::times` throws a `LogicError` when the locale tag is not valid.

**Good to know:** a `Core\Time\TimeOfDay` has no date and no time zone. To write the time of a
`Core\Time\DateTime`, call `$value->timeOfDay()` first.
