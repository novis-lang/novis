Moves this instant back by an exact `Duration` and returns the new instant.

The original instant does not change. `minus(1h)` returns the moment exactly 3600 seconds earlier,
and `minus(24h)` returns the moment exactly 24 hours earlier. An instant has no time zone, so a
change to or from summer time does not change the result. A negative duration moves the instant
forward.

The result must be between the years -9999 and 9999. Outside that range, `minus` throws a
`RuntimeError`.

To count calendar days or months, use a `DateTime` in a time zone instead.
