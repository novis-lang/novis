Moves a time of day back by a number of hours, minutes, seconds or smaller units, and returns the
new time.

The original time does not change. A `TimeOfDay` has no date, so the result wraps around at
midnight: 00:15 minus 30 minutes is 23:45. A negative count moves the time forward, so `minus`
gives the same result as `plus` with the opposite count.

The unit must be `Unit::Hour` or smaller. `Unit::Day` and the larger units throw a `LogicError`,
because a time of day has no days to move. A count too large for its unit throws a
`RuntimeError`.
