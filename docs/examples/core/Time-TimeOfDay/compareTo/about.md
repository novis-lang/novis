Checks which of two times of day comes first, and returns `-1`, `0` or `1`.

The result is `-1` when the time comes earlier in the day than the other one, `0` when both are the
same time, and `1` when it comes later. The check uses every part of the time, down to the
nanosecond. A `TimeOfDay` has no date, so 00:10 always comes before 23:50.

The operators `<`, `>`, `<=`, `>=` and `<=>` use `compareTo` too, so you can write `$a < $b`.
`==` compares the time too. Two times with the same hour, minute, second and nanosecond are equal,
even when your program made them separately.
