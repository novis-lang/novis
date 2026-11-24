Divides one number by another and rounds the answer to as many digits as you ask for.

`Core\Decimal::divideRounded` takes four arguments: the value, the divisor, how many digits to keep
after the point, and how to round. `100` divided by `3` at two digits is `33.33`. The result always
has as many digits after the point as you asked for, so `100` divided by `4` at two digits is
`25.00`. Dividing by zero throws an `ArithmeticError`, and so does a scale above 28.

Use it where rounding is part of the rule your program follows. Examples are a bill split between
people, a price converted into another currency, and a rate applied to an amount. The fourth argument is a
`Core\RoundMode` case. It says how to round a value that is exactly between two results.
`Core\Decimal::divideExact` is the division to use when the answer has to come out even.

**Good to know:** the `/` operator also rounds. A value exactly between two results goes to the
even one, and you cannot change that. Use `divideRounded` when your program must choose the number
of digits or the rounding.
