Equality answers one question — are these two values the same? — and `==` and `!=` are the whole of
it. Neither one converts anything, so a number is never quietly equal to text: `1 == "1"` does not
compile at all, and the compiler points at the comparison and tells you to convert one side
yourself. Numbers of different kinds do compare, because they are one domain, so `1 == 1.0` is true.
Text compares character by character, arrays compare their contents, and two objects are equal only
when they are the same object, so a copy is never equal to the original it was made from.

`<`, `<=`, `>`, `>=` and `<=>` put two values in order. They take numbers, two `bool`s, and two
objects of a class that implements `Comparable`. Text orders through `Core\Str::compare` instead,
because there is more than one sensible answer for it. `<=>` gives back `-1`, `0` or `1`, which is
exactly what sorting needs.

**The examples below** take these in turn: what to do when a number arrives as text, how arrays,
objects and a missing value compare, and putting two delivery offers in order by price.
