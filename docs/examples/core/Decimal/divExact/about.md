Divides one number by another and throws an error when the answer is not exact.

`Core\Decimal::divExact` returns the quotient when it can be written as a `decimal`, and throws an
`ArithmeticError` when it cannot. `10 / 4` is `2.5`, so that call returns `2.5`. `10 / 3` is
`3.333…` with no end, so that call throws. Dividing by zero throws as well.

Use it where your program already assumes the division comes out even. Splitting a total between a
known number of people, converting a quantity between two units, and working out a unit price from a
pack price are all cases where a rounded answer would be a silent mistake. `Core\Decimal::divRound`
is the division to use when rounding is what you want. It takes how many digits to keep and how to
settle a value between two neighbours.

**Good to know:** the answer keeps only the digits it needs. `10 / 4` returns `2.5`, not `2.50`,
even when the numbers you divided were written with two digits.
