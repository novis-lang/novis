The usual arithmetic — `+ - * / % **` — on whole numbers, on fractions and on exact decimal numbers.

Both sides have to be numbers of the same kind. Nothing is converted for you, so `"3" * 2` does not
compile and you write `($s as int) * 2` instead. Dividing two whole numbers gives a whole number
when it comes out even and a fraction when it does not, which is why `7 / 2` is `3.5`. A result too
large for a whole number throws an `ArithmeticError`, and so do division by zero, `%` by zero and a
negative exponent. Nothing wraps around to a negative number, and nothing turns into a fraction
without you asking.

**Good to know:** use `decimal` for money. `float` is the fast kind and cannot store `0.1` exactly,
so ten additions of `0.1` do not come to `1.0`. `decimal` counts in digits and does.

**The examples below** show everyday arithmetic first, then the errors that stop a wrong number
being used, then a basket priced in `decimal`.
