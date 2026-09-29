Moves this instant forward by an exact `Duration` and returns the new instant.

The original instant does not change. `plus(1h)` returns the moment exactly 3600 seconds later,
and `plus(24h)` returns the moment exactly 24 hours later. An instant has no time zone, so a
change to or from summer time does not change the result. On a local clock, the same step can
show 23 or 25 hours on the day the clock changes. A negative duration moves the instant back.

The result must be between the years -9999 and 9999. Outside that range, `plus` throws a
`RuntimeError`.

To add calendar days or months, use a `DateTime` in a time zone instead.
