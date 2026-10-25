A string is never a `callable`. `callable $f = "strlen";` is the error `E0418`. Write a method
reference such as `Core\Str::length(...)` instead.

Any class instance can be assigned to an `object` variable. You can read a property through it by
name, and the result is `mixed`. A property the object does not have throws a `RuntimeError`.
`$o is Cell` tests the class, and `$o as Cell` converts to it. If the object is another class, `as`
throws a `RuntimeError`. `as ?Cell` is the error `E0473`, so use `is` to test first.

A `class<T>` variable contains a class, not an instance. `T` is a class or an interface, and every
class in the variable is `T` or extends or implements it. A `class<Dog>` can be assigned to a
`class<Animal>`. Two class references are equal when they are the same class.

`as class<T>` is the only way to get a class reference. A class name in quotes is checked when the
program compiles, and a name outside the hierarchy of `T` is the error `E0708`. A name in a variable
is checked when the program runs. A wrong name throws a `RuntimeError`, and `as ?class<T>` gives
`null` for it. `$cls as string` gives the class name.

Only three places take a class reference: `new $cls()`, `$cls::f()` and `$x is $cls`. A `string` in
any of the three is the error `E0496`. `Dog::class` is a `string`, the class name. Comparing a
`string` with a `class<T>` is the error `E0466`.

A shape cannot be written in front of a variable, because a `{` at the start of a statement opens a
block. `{ x: int } $p = …;` is the error `E0134`. Name the shape with `type` first.

A value assigned to a shape needs every field the shape names, with the type it gives. A missing
field or a field of another type is the error `E0401`. You can assign a new value to an existing
field of an anonymous object. Adding a field it does not have throws a `RuntimeError`.

`as Point` checks the type of each field too. `{ x: 5, y: "six" } as Point` throws a `RuntimeError`,
and `as ?Point` gives `null` for it.
