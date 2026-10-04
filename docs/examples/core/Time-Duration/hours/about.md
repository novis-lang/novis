Creates a duration of a given number of hours. Each hour is exactly 3600 seconds.

Use it when the number of hours is known only while the program runs, for example when it comes
from a setting. When the number is fixed, write the duration directly in the code,
such as `72h`. A negative number
gives a negative duration. The number can be up to 2,562,047 hours, which is about 292 years. A
larger number throws a `RuntimeError`.

**Good to know:** adding `72h` to a time is not always the same as adding three calendar days. On
the day the clocks change for daylight saving time, the clock shows a different hour afterwards.
