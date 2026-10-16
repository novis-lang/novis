Divides one `float` by another, like the `/` operator. The difference is what happens when you
divide by zero. The `/` operator throws an `ArithmeticError`.
`Core\Math::fdiv` returns a value instead.

When you divide a number by zero, the result is `INFINITY` or `-INFINITY`. The sign comes from
the signs of both numbers together, so `1.0` divided by `-0.0` is `-INFINITY`. When you divide
zero by zero, the result is `NaN` (a value that means "not a number").

Use it when a zero divisor is a normal case in your data and you want to handle it after the
division. `Core\Math::isFinite` and `Core\Math::isNan` test the result.

**The examples below** show a few divisions, then the difference from the `/` operator, then a
report that works out a rate for a page with no visits.
