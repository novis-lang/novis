Creates a duration of a given number of microseconds. A microsecond is one millionth of a second.

Use it when the number of microseconds is known only while the program runs, for example when it
comes from a measurement or a log file. When the number is fixed, write a literal such as `250us`
instead. A negative number gives a negative duration. The number can be up to
9,223,372,036,854,775 microseconds, which is about 292 years. A larger number throws a
`RuntimeError`.

`toString` shows the duration in the largest units that fit. So 1500 microseconds prints as
`1ms500us`.
