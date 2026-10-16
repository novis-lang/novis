Creates a duration of a given number of minutes. Each minute is exactly 60 seconds.

Use it when the number of minutes is known only while the program runs, for example when the
length of a meeting or a session timeout comes from a setting. When the number is fixed, write the
duration directly in the code, such as `15m`. A negative number gives a negative duration. The
number can be up to 153,722,867 minutes, which is about 292 years. A larger number throws a
`RuntimeError`.

`toString` shows the duration in the largest units that fit. So 90 minutes prints as `1h30m`.
