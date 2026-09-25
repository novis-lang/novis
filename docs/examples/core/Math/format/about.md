`Core\Math::format` writes a number as text for a person to read. You choose how many decimals to
write, the text between the whole part and the decimals, and the text between groups of three
digits. This replaces PHP's `number_format`.

The number can be an `int`, a `float` or a `decimal`. It is rounded to the number of decimals you
ask for, and a half is rounded away from zero, so `2.5` is written as `3`. The digits are not
grouped unless you give a `groupSeparator`. A negative number starts with `-`, and a number that
rounds to zero has no `-`.

A `float` cannot store most decimal fractions exactly. `1.005` is stored as a number slightly below
it, so it is written as `1.00` with two decimals. Use a `decimal` when the digits must be exact,
such as for money.

`INFINITY` and `NaN` have no digits, so they throw a `RuntimeError`.

**The examples below** show decimals and rounding, then separators for different countries, then a
price list.
