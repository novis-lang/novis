Cuts a number to fewer digits by dropping the ones it does not keep.

`Core\Decimal::truncate` never changes a digit it keeps. `12.39` stays `12.39` even when the next
digit is a `9`. The second argument says how many digits after the point to keep, and leaving it out
gives a whole number. The answer is exact, because it is a `decimal` and not a floating-point number.

Reach for it when the extra digits do not matter: a percentage in a progress line, a measurement
written into a column of fixed width, or a value you must always print at or below its real size.

**Good to know:** `truncate` always moves towards zero. On a positive number it agrees with
`Core\Decimal::floor`, and on a negative one it agrees with `Core\Decimal::ceil`. So
`truncate(-2.7)` is `-2`, while `floor(-2.7)` is `-3`.
