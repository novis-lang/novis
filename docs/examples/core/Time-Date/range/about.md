Returns a list of the dates from a first date `$from` to a last date `$to`. By default, both dates are
in the list. `Date::range(Date::at(2026, 1, 30), Date::at(2026, 2, 1))` returns `2026-01-30`,
`2026-01-31` and `2026-02-01`.

By default, the list has one date for each day. The `step` option sets the distance between two
dates, such as `{ step: Unit::Week }`. It can be `Unit::Day`, `Unit::Week`, `Unit::Month`,
`Unit::Quarter` (three months) or `Unit::Year`. With a step of a week or more, the last date in the
list can be before `$to`.

Each date is counted from `$from`. With a step of one month, every date has the same day as
`$from`. When a month is too short, the date is the last day of that month. From `2026-01-31`, the
dates are `2026-02-28`, `2026-03-31`, `2026-04-30` and so on.

When `$from` is after `$to`, the list goes backward. A very long list uses a lot of memory. If it
needs more than the memory limit of the program, the program stops.
