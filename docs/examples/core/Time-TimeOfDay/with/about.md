Returns a copy of a time of day with some of its parts changed.

You name the parts to change: `hour`, `minute`, `second` and `nanos` (the nanoseconds inside the
second). The parts you leave out keep their value. With no parts at all, the result is the same
time. The original time does not change.

Each new value must be in the range of its part: 0 to 23 for the hour, 0 to 59 for the minute and
the second, and 0 to 999999999 for the nanoseconds. A value outside its range throws a
`RuntimeError`, and `with` does not move the time into the next hour or day.
