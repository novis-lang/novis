Makes a time of day from its parts: an hour, a minute and, if you need them, a second and
nanoseconds.

A `TimeOfDay` is a time on a clock, such as 10:30. It has no date and no time zone. Use it for
times that repeat every day, like opening hours or the time a daily job runs.

The hour goes from 0 to 23, and the minute and the second go from 0 to 59. The second and the
nanoseconds are optional, and their default is 0. When a part is outside its range, `at` throws a
`RuntimeError`. So 24:00 is not a time of day: midnight is hour 0 and minute 0.
