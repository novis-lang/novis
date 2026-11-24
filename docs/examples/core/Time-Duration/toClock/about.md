Returns a duration as text in hours and minutes, such as `1:30`. A timesheet shows hours this way.

The hours do not stop at 24, so 25 hours and 30 minutes gives `25:30`. Five minutes gives `0:05`.
A negative duration starts with `-`, such as `-1:15`. The method never throws.

Pass `{ seconds: true }` to add the seconds, such as `25:30:00`. By default, the text ends at the
minutes. The seconds are cut off, not rounded, so 119 seconds gives `0:01`. A negative duration that
is cut off to zero gives `0:00`.

`Core\Time\Duration::fromClock` reads this text and returns a duration.

The examples show a few durations as text, the seconds option with a negative duration, and the
hours of a weekly timesheet.

**Good to know:** `toString` writes the same duration as `1h30m`, the way you write it in code.
