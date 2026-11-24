Returns the quarter of the year that contains this date and time, from `1` to `4`.

Quarter `1` is January to March, `2` is April to June, `3` is July to September and `4` is
October to December. The month is the month in the value's own time zone. One moment can be
31 March in New York and 1 April in Berlin, so the result can depend on the time zone.

Use `quarter` to group orders, invoices or reports by quarter. The pattern letter `Q` in `format`
prints the same number as text.
