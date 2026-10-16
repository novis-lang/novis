Adds up every value in an array and gives you the total.

The type of the total follows the values it meets. Whole numbers added to whole numbers stay whole.
One `float` anywhere makes the total a `float`. One `decimal` anywhere makes the total a `decimal`,
which is the type to use for money.

A total that grows past the range a whole number covers throws an error. It does not wrap around and
does not turn into a `float`, so a total is always exact.

A `float` and a `decimal` in the same array throw an error as well. There is no type that holds both
of them, so there is no total to give you.

An array with no entries gives you 0.

The examples show a list of whole numbers, prices added up as `decimal`, and the total of a shopping
cart worked out from its lines.
