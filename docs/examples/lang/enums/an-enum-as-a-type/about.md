Uses an enum's name as a type.

An enum is a type like `int` or `string`, and you can write its name anywhere a type is written: a
property, a parameter, a return type, the element type of an `array<Suit>`, the variable of a
`foreach` loop, and the type a generator gives with `Iterator<Suit>`. Everything declared that way
accepts the cases of that enum and no other value. A number or a string there does not compile.

An array of type `array<Suit, T>` uses the cases of `Suit` as its keys. The next section, *An enum
as an array key*, shows how.

**The examples below** show the type on a property, a parameter and a return value, then an array of
cases and two loops over one, then a list of orders that reads a text label from a table.
