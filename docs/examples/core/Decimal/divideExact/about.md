Divides one number by another and throws an error when the answer is not exact.

`Core\Decimal::divideExact` returns the result when it can be written as a `decimal`, and throws
an `ArithmeticError` when it cannot. `10 / 4` is `2.5`, so that call returns `2.5`. `10 / 3` is
`3.333…` with no end, so that call throws. Dividing by zero throws as well.

Use it where your program already assumes the division comes out even. Examples are splitting a
total between a known number of people, and converting a quantity between two units. Another is
working out a unit price from a pack price. In each case, a rounded answer would be a mistake that
nobody sees.
`Core\Decimal::divideRounded` is the division to use when rounding is what you want. It takes how
many digits to keep, and how to round a value that is exactly between two results.

**Good to know:** the result has only the digits it needs. `10.00 / 4` returns `2.5`. It does not
return `2.50`.
