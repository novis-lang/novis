A string is never a `callable`. `callable $f = "strlen";` is the error `E0418`. Write a method
reference such as `Core\Str::length` instead.

Any class instance can be assigned to an `object` variable. You can read a property through it by
name, and the result is `mixed`. A property the object does not have throws a `RuntimeError`.
`$o is Cell` tests the class, and `$o as Cell` converts to it. If the object is another class, `as`
throws a `RuntimeError`. `as ?Cell` is the error `E0473`, so use `is` to test first.

A `class<T>` variable contains a class, not an instance. `T` is a class or an interface, and every
class in the variable is `T` or extends or implements it. A `class<Dog>` can be assigned to a
`class<Animal>`. Two class references are equal when they are the same class.

There are two ways to get a class reference. Where a `class<T>` is expected, `Dog::class` is a class
reference. That place can be a variable, a parameter, a return value or a default value.
`self::class`, `parent::class`, `static::class` and `$obj::class` work the same way. The compiler
checks the class. A class that is not `T` and does not extend or implement `T` is the error
`E0708`. Everywhere else, `Dog::class` is a `string`, the class name.

The other way is `as class<T>`, for a class name in a string. A class name in quotes is checked when
the program compiles. A name in a variable is checked when the program runs. A wrong name throws a
`RuntimeError`, and `as ?class<T>` gives `null` for it. A `string` where a `class<T>` is expected
does not compile. The error message shows how to write the `as`. `$cls as string` gives the class
name.

Only three places take a class reference: `new $cls()`, `$cls::f()` and `$x is $cls`. A `string` in
any of the three is the error `E0496`. Comparing a `string` with a `class<T>` is the error `E0466`.

A shape cannot be written in front of a variable, because a `{` at the start of a statement opens a
block. `{ x: int } $p = …;` is the error `E0134`. Name the shape with `type` first.

A value assigned to a shape needs every field the shape names, with the type it gives. A missing
field or a field of another type is the error `E0401`. You can assign a new value to an existing
field of an anonymous object. Adding a field it does not have throws a `RuntimeError`.

`as Point` checks the type of each field too. `{ x: 5, y: "six" } as Point` throws a `RuntimeError`,
and `as ?Point` gives `null` for it.

A `///` comment above a field of a shape describes that field. The editor shows the text when you
type the field's name in an object for that shape, or point at it.
