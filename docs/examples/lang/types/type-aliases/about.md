A `type` alias gives a type a short name. After `type UserId = uint;`, `UserId` and `uint` are the
same type. A value of one is a value of the other, so you never convert between them.

You write an alias at the top level of a file, or inside a class, interface or enum. An alias
inside a class belongs to that class. Outside the class, you write the class name first:
`Basket::Lines`.

An alias cannot be a second name for one class, because a class already has a name. Use an alias
for a type that is long to write: a union, a nested array, or the fields of an object a method
returns.

An alias exists only while the program compiles. It has no cost while the program runs.

Example 04 names a nullable class and a union. Example 05 shows that an alias of one class does not
compile.
