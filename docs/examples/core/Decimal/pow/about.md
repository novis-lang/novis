Multiplies a number by itself as many times as you say, and keeps every digit of the answer.

`Core\Decimal::pow` takes the base and the number of times to multiply. `Core\Decimal::pow(2, 10)`
is `1024`, and an exponent of `0` gives `1`. The exponent is a whole number and is never negative.
The answer is exact: `2.50` squared is `6.2500`, because the digits after the point of both numbers
are added together. A power wider than a `decimal` holds throws an `ArithmeticError`.

Use it where the value is money or a measurement and a rounded answer would be wrong: interest over
a number of years, a growth factor applied again and again, a unit converted by a scale factor. The
`**` operator has no answer for a `decimal` base, so this method is how a `decimal` is raised.

**Good to know:** the answer of a base with digits after the point gets wide quickly.
`Core\Decimal::pow(1.05, 14)` already has 28 digits after the point, which is all a `decimal` holds,
and 15 throws. Round the power with `Core\Decimal::divRound` once you are done with it.
