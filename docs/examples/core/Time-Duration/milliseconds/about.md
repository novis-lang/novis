Creates a duration of a given number of milliseconds. A millisecond is one thousandth of a second.

Use it when the number of milliseconds is known only while the program runs, for example when a
timeout comes from a setting or a delay grows with each retry. When the number is fixed, write the
duration directly in the code, such as `250ms`. A negative number gives a negative duration. The
number can be up to 9,223,372,036,854 milliseconds, which is about 292 years. A larger number throws
a `RuntimeError`.

`toString` shows the duration in the largest units that fit. So 1500 milliseconds prints as
`1s500ms`.
