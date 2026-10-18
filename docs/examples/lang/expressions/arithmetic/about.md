The arithmetic operators `+ - * / % **` work on the four number types: `int`, `uint`, `float` and
`decimal`.

Both sides must be numbers. `"3" * 2` does not compile, so you write `($s as int) * 2`. A whole
number and a `float` give a `float`, so `3 + 0.5` is `3.5`. An `int` and a `uint` do not mix, and a
`decimal` and a `float` do not mix either. Dividing two whole numbers gives a whole number when the
result is exact. Otherwise it gives a `float`, so `7 / 2` is `3.5`. A result too large for a whole
number throws an `ArithmeticError`. A division by zero, `%` by zero and a negative power of a whole
number throw it too.

**Good to know:** use `decimal` for money. A `float` cannot store `0.1` exactly, so ten additions of
`0.1` do not give `1.0`. A `decimal` stores digits, and the total is exact.

**The examples below** show everyday arithmetic, the errors arithmetic throws and a basket priced in
`decimal`. Then they show an `int` and a `uint` in one expression, the operators `++` and `--`, and
the bitwise operators.
