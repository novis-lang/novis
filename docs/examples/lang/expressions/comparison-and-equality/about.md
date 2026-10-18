`==` tests whether two values are equal, and `!=` tests whether they are different. These are the
only two equality operators.

Neither operator converts a value. `1 == "1"` does not compile, and the error tells you to convert
one side first. Numbers of different types do compare, so `1 == 1.0` is true. Two strings are equal
when they have the same characters. Two arrays are equal when they have the same keys and values in
the same order. Two objects are equal only when they are the same object, so a copy is not equal to
the original.

`<`, `<=`, `>`, `>=` and `<=>` put two values in order. They work on numbers, on two `bool` values,
and on two objects of a class that implements `Comparable`. They do not work on strings. Use
`Core\Str::compare` to order two strings. On numbers, `<=>` returns `-1`, `0` or `1`.

**The examples below** show a number that arrived as text, then arrays, objects and a missing value,
then two delivery offers put in order by price. The last two show the errors for an `int` compared
with a string and for `<` on two strings.
