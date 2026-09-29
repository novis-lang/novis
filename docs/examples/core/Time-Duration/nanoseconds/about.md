Creates a duration of a given number of nanoseconds. A nanosecond is one billionth of a second. It is
the smallest unit a duration has, so no part of the number is lost.

Use it when a timer or a log gives a time in nanoseconds, or when the number is known only while the
program runs. When the number is fixed, write a literal such as `250ms` instead. A literal and this
method give the same duration for the same length.

Every `int` is a valid number of nanoseconds, so this method never throws. A negative number gives a
negative duration. The largest duration is about 292 years in each direction. A sum or a product
that goes past it throws a `RuntimeError`.
