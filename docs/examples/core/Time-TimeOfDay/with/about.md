Returns a copy of a time of day with some of its parts changed.

You name the parts to change: `hour`, `minute`, `second` and `nanoseconds` (the nanoseconds inside the
second). The parts you leave out keep their value. With no parts at all, the result is the same
time. The original time does not change.

Each new value must be in the range of its part. The hour is 0 to 23, and the minute and the second
are 0 to 59. The nanoseconds are 0 to 999999999. A value outside its range throws a
`RuntimeError`. The time does not move into the next hour or day.
