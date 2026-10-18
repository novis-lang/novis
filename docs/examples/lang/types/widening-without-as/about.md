Some values are converted to a wider type without `as`. This happens when you assign a value, pass
an argument, return a value or put an element in an array.

An `int` or a `uint` is converted to a `float`. A value is converted to a nullable type, to a union
that contains its type, and to `mixed`. An object is converted to a parent class, to an interface its
class implements, and to `object`. A single-value type such as `"read"` is converted to `string`.

Only the conversion to `float` changes the value. It is checked while the program runs. A number
above 2^53 cannot be stored exactly in a `float`, so the program throws an `ArithmeticError`.

No other conversion happens without `as`. A `string` to a number, a number to a `string`, and an
`int` to a `uint` each need `as`.

**The examples below** show a whole number used as a `float`, the wider types, a price list with
both kinds of number, an anonymous object at a shape, and a program that assigns an `int` to a `uint`
and does not compile.
