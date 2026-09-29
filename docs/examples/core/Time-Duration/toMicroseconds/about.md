Returns the length of a duration as a number of whole microseconds. A microsecond is one millionth
of a second, or 1000 nanoseconds.

Use it when another tool expects microseconds, for example a database log that shows how long each
query took. The method counts only whole microseconds. The part that is shorter than one
microsecond is ignored, so 1500 nanoseconds gives `1`. A negative duration gives a negative number
in the same way: -1500 nanoseconds gives `-1`. Every duration fits, so this method never throws.

Use `toNanoseconds` when you need the exact length.
