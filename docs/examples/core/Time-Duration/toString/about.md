Returns a duration as text, in the same syntax you use to write a duration in code, such as `1h30m`
or `250ms`.

The largest unit comes first, and a unit with a count of zero is left out. So 90 seconds gives
`1m30s`, and 1.5 seconds gives `1s500ms`. A duration of zero gives `0s`. A negative duration gives the
same text with a `-` in front, such as `-1h`. The method never throws.

Use it to show a duration to a person, or to write it to a log or a setting. Text from a duration
that is not negative can be read back with `Core\Time\Duration::parse`, and the result is the same
duration. `parse` throws an error for text that starts with `-`.

The examples show a few durations as text, a negative duration, and a log line that says how long a
job took. A duration also becomes this text when you put it inside a string with `"{$took}"`.
