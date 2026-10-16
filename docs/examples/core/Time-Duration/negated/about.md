Returns a duration of the same length that points the other way. A positive duration becomes
negative, a negative one becomes positive, and zero stays zero. The duration you call it on does not
change.

Use it to undo a step, for example to move a time back by an offset that you stored as a positive
duration.

One duration has no opposite: the smallest one, `Core\Time\Duration::nanoseconds` of the smallest
`int`. It is one nanosecond longer than the largest positive duration. For this duration, `negated`
throws a `RuntimeError`. Every other duration can be negated.
