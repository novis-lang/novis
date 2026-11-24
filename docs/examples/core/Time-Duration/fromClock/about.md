Reads a duration from text in hours and minutes, such as `1:30`, or in hours, minutes and seconds,
such as `1:30:15`. Use it for a time a person types into a form.

The hours may have any number of digits, so `25:30` is 25 hours and 30 minutes. The minutes and the
seconds have two digits each. A `-` at the start makes the duration negative, such as `-1:15`.

`fromClock` throws a `ParseError` in these cases:

- The text is not in this form. Spaces and a `+` sign are not allowed.
- The minutes or the seconds are larger than `59`.
- The duration is longer than `2562047:47:16`, the longest `Duration`.

`fromClock` can read every text that `Core\Time\Duration::toClock` returns. To read `1h30m`, the
syntax you use in code, call `Core\Time\Duration::parse`.

The examples read a few times, show which texts throw an error, and check the times a person types
into a form.
