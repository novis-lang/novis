Moves a time of day forward by a number of hours, minutes, seconds or smaller units, and returns
the new time.

The original time does not change. A `TimeOfDay` has no date, so the result wraps around at
midnight: 23:30 plus one hour is 00:30. A negative count moves the time back.

The unit must be `Unit::Hour` or smaller. `Unit::Day` and the larger units throw a `LogicError`,
because a time of day has no days to move. A count too large for its unit throws a
`RuntimeError`. To move across days, use a `DateTime` instead.
