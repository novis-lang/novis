Divides one number by another and rounds the answer to as many digits as you ask for.

`Core\Decimal::divRound` takes four arguments: the value, the divisor, how many digits to keep after
the point, and how to round. `100` divided by `3` at two digits is `33.33`. The answer always carries
the scale you named, so `100` divided by `4` at two digits is `25.00`. Dividing by zero throws an
`ArithmeticError`, and so does a scale above 28.

Use it where rounding is part of the rule your program follows: a bill split between people, a price
converted into another currency, a rate applied to an amount. The fourth argument is a
`Core\RoundMode` case, and it decides where a value that sits exactly between two neighbours goes.
`Core\Decimal::divExact` is the division to use when the answer has to come out even.

**Good to know:** the `/` operator rounds as well, but it always sends a value between two
neighbours to the even one, and it takes no arguments. Name the scale and the mode here when that
choice belongs to your program.
