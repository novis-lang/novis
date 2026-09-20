Returns this number as a `decimal`.

A `decimal` is the type for money, and for any number where rounding is not allowed. It holds a
value exactly, and it has a fixed size: it reaches from -79228162514264337593543950335 to
79228162514264337593543950335. A `Core\BigInt` has no size limit, so not every number fits a
`decimal`.

`toDecimal` returns the `decimal` equal to this number. The result has no digits after the decimal
point, and it is exact. When the number is outside the range above, `toDecimal` throws an
`ArithmeticError`. It never rounds a number to make it fit.

Use this where a whole-number calculation has to continue in money arithmetic: add the parts up as a
`Core\BigInt`, where no total is ever too large, then convert once and divide or multiply as a
`decimal`.
