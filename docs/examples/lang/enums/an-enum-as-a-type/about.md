Uses an enum's name as a type.

An enum is a type like `int` or `string`, and you can write its name anywhere a type is written: a
property, a parameter, a return type, the element type of an `array<Suit>`, the variable of a
`foreach` loop, and the type a generator gives with `Iterator<Suit>`. Everything declared that way
accepts the cases of that enum and no other value, so a number or a text cannot reach it.

A case is not an array key. Write `$case as int` to get the number behind the case, and use that
number as the key. The array then works as any array with whole-number keys does.

**The examples below** show the type on a property, a parameter and a return value, then an array of
cases and two loops over one, then a list of orders that reads a text label from a table.
