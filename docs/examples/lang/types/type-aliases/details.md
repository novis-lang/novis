An alias name is written in PascalCase. `type user_id = uint;` is the error `E0110`.

An alias at the top level of a file can be used anywhere in that file, also above its declaration.
An alias inside a method body or a block, such as an `if`, is the error `E0233`.

An alias inside a class, interface or enum has no visibility. `public type Lines = …;` is the error
`E0133`. Inside its class you write the short name. Everywhere else you write `Owner::Name`.

A child class does not inherit an alias. `Lines` in a child class of `Basket` is the error `E0303`.
Write `Basket::Lines` there.

An alias of a single class, interface or enum is the error `E0307`. `?Foo`, a union, an object
shape and `array<Foo>` are allowed.

An alias may use another alias, such as `type Ids = array<Id>;`. An alias that contains itself is
the error `E0310`, such as `type Tree = array<Tree>;`. Use a class for a type that contains itself.

Two declarations with the same name are the error `E0304`. This is also true for an alias and a
class with the same name.
