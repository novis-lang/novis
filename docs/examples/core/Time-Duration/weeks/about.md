Creates a duration of a given number of weeks. Each week is exactly 7 days of 24 hours, which is
168 hours.

Use it when the number of weeks is known only while the program runs, for example when it comes from
a setting. When the number is fixed, write the duration directly in the code,
such as `2w`. A negative number gives a
negative duration. The number can be up to 15,250 weeks, which is about 292 years. A larger number
throws a `RuntimeError`.

**Good to know:** a calendar week is not always 168 hours. In a week where the clocks change for
daylight saving time, it has one hour more or less. To move a date to the same time one calendar
week later, use `plus` on a `Core\Time\DateTime` with `Core\Unit::Week`.
