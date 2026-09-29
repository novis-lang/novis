Returns the length of a duration as a number of whole milliseconds. A millisecond is one thousandth
of a second.

Use it when another program expects milliseconds. Many do: a timer in a browser script, a field in
a JSON reply, or the retry time of an event stream. The method counts only whole milliseconds. The
part that is shorter than one millisecond is ignored, so 999 microseconds gives `0`. A negative
duration gives a negative number in the same way. Every duration fits, so this method never throws.

Use `toNanoseconds` when you need the exact length.
