In `property<T>`, `T` must be a class with at least one public property. An interface, an enum or a
type such as `int` is the error `E0799`.

A name written directly in the code is checked while compiling. `"emial" as property<User>` is the
error `E0405`. A string literal at a parameter typed `property<User>` or `?property<User>` is
converted to a key while compiling. It needs no `as`, and a wrong name is also `E0405`. In a
variable, a return value or an array, a string literal stays a `string`.

`as` gives the same `RuntimeError` for an unknown name and for a `private` or `protected` property.
The message lists the public properties of the class.

`$obj->$key` and `$obj->{$expr}` read or write the property that the key names. With a variable of
type `string` there, the program does not compile (`E0235`).

A read through a key has the type of every property the key can name, as a union. For a class with
a `string` property and an `int` property, the type is `string|int`.

A write through a key is checked twice. While compiling, the value must have the type of at least
one property. `true` written through a key of a class with only `string` properties is `E0401`.
While the program runs, the value must have the type of the property the key names. If it does not,
the write throws a `RuntimeError`. A `tainted` or `secret` value must fit every property the key can
name. If one property is a plain `string`, writing a `tainted string` is `E0401`.

If the class has a public `readonly` property, a write through a key does not compile (`E0782`). A
`private readonly` property does not stop the write.

A `property<Animal>` key works on a `Dog` when `Dog` extends `Animal`. You can also store it in a
variable of type `property<Dog>`. A `property<Dog>` key on an `Animal` is the error `E0235`.

`$key as string` gives the property name. Two keys are equal when they name the same property. A
key written in the code and a key converted from a variable are equal when the name is the same.
