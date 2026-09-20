Multiplies all the values of an array together and gives you the result. It replaces PHP's
`array_product`.

The values are multiplied one at a time, and the type of the result follows what it meets. Whole
numbers multiplied by whole numbers stay whole. One `float` anywhere makes the result a `float`.
One `decimal` anywhere makes the result a `decimal`, which is the type to use for money.

A result that grows past the range a whole number covers throws an error. It does not wrap around
and it does not turn into a `float`. Multiplication grows fast, so a few large numbers are enough
to reach that point.

A `float` and a `decimal` in the same array throw an error as well. There is no type that holds
both of them, so there is no result to give you.

An array with no entries gives you 1. A single value of 0 anywhere makes the result 0.

The examples show a list of whole numbers, three discounts applied one after the other, and the
number of variants a shop article has.
