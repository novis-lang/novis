Returns the length of a duration as a number of whole seconds.

Use it when a header, a setting or another service expects seconds. For example, the `max-age`
value of a `Cache-Control` header and the `Retry-After` header are both numbers of seconds. The
method counts only whole seconds. The part that is shorter than one second is ignored, so a
duration of 999 milliseconds gives `0` and 1.5 seconds gives `1`. A negative duration gives a
negative number in the same way. Every duration fits, so this method never throws.

Use `toMilliseconds` or `toNanoseconds` when the part after the last whole second matters.
