Returns the quarter of the year that contains a date, as a number from `1` to `4`.

A quarter is three months. Quarter `1` is January to March, `2` is April to June, `3` is July to
September and `4` is October to December. The result is a `uint`, and `quarter` never throws an
error.

Use it to group sales or invoices by quarter. `startOf(Unit::Quarter)` and `endOf(Unit::Quarter)`
return the first and the last day of the quarter.

**Good to know:** `format` with `QQQ` prints the quarter as text, such as `Q2`.
