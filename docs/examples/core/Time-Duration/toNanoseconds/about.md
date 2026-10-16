Returns the length of a duration as a number of nanoseconds. A nanosecond is one billionth of a
second. A duration stores its length in nanoseconds, so this number is exact and no part of the
length is lost.

Use it when you need the most precise number, for example to save a duration in a database column
or to compare it with a timer that counts in nanoseconds. A negative duration gives a negative
number. Every duration fits in an `int`, so this method never throws.

`toMicroseconds`, `toMilliseconds` and `toSeconds` return coarser numbers. They count only whole
units and ignore the rest.
