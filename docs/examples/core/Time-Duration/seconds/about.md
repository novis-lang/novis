Creates a duration of a given number of seconds.

Use it when the number of seconds is known only while the program runs, for example when a cache
lifetime or a timeout comes from a setting, or from another service. When the number is fixed, write the
duration directly in the code, such as `30s`. A negative number gives a negative duration. The
number can be up to 9,223,372,036 seconds, which is about 292 years. A larger number throws a
`RuntimeError`.

`toString` shows the duration in the largest units that fit. So 90 seconds prints as `1m30s`.
