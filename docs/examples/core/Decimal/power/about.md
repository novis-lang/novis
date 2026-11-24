Multiplies a number by itself as many times as you say, and keeps every digit of the answer.

`Core\Decimal::power` takes the base and the number of times to multiply.
`Core\Decimal::power(2, 10)` is `1024`, and an exponent of `0` gives `1`. The exponent is a whole
number and is never negative. The answer is exact: `2.50` squared is `6.2500`, because the digits
after the point of both numbers are added together. When the result does not fit in a `decimal`,
the call throws an `ArithmeticError`.

Use it where the value is money or a measurement, and a rounded answer would be wrong. Examples are
interest over a number of years, a growth factor applied many times, and a unit converted by a scale
factor. The
`**` operator does not work with a `decimal` base, so use this method to raise a `decimal`.

**Good to know:** when the base has digits after the point, the result gets long quickly.
`Core\Decimal::power(1.05, 14)` already has 28 digits after the point. That is the most a `decimal`
can store, so an exponent of 15 throws. To go further, round the result with `Core\Decimal::round`
and raise that.
