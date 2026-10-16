Shows this instant on the calendar and clock of a time zone, and returns it as a `DateTime`.

An instant is one moment, and it has no time zone. To get a date, an hour or a weekday, you need a
zone, and `in` is the way to add one. `in(Core\Time\Zone::of("Europe/Berlin"))` returns the date and
time a clock in Berlin shows at that moment.

The zone uses the offset it has at that moment. The same zone can give `+01:00` in winter and
`+02:00` in summer. The instant itself does not change, so the `DateTime` and the instant are
always the same moment.

The examples show one moment in three cities, the change to summer time, and order times shown in
each customer's own zone.
