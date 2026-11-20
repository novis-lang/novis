In `property<T>`, `T` must be a class with at least one public property. An interface, an enum or a
type such as `int` is the error `E0799`.

A name written directly in the code is checked while compiling. `"emial" as property<User>` is the
error `E0405`. You can pass a string literal to a parameter of type `property<User>` or
`?property<User>` without `as`. The compiler converts it to a key, and a wrong name is `E0405`. In a
variable, a return value or an array, a string literal stays a `string`.

`as` gives the same `RuntimeError` for an unknown name and for a `private` or `protected` property.
The message lists the public properties of the class.

`$obj->$key` and `$obj->{$expr}` read or write the property that the key names. With a variable of
type `string` there, the program does not compile (`E0235`).

A read through a key has the type of every property the key can name, as a union. For a class with
a `string` property and an `int` property, the type is `string|int`.

A write through a key is checked when the program compiles and again when it runs. When it compiles,
the value must have the type of at least one property. `true` written through a key of a class with
only `string` properties is `E0401`. When it runs, the value must have the type of the property the
key names. If it does not, the write throws a `RuntimeError`.

A `tainted` value is input from outside the program, such as a request. A `secret` value, such as a
password, must not be printed. Such a value can only be written through a key when every property
the key can name allows it. If one property is a plain `string`, writing a `tainted string` is `E0401`.

If the class has a public `readonly` property, a write through a key does not compile (`E0782`). A
`private readonly` property is never named by a key, so a write through a key still compiles.

A `property<Animal>` key works on a `Dog` when `Dog` extends `Animal`. You can also store it in a
variable of type `property<Dog>`. A `property<Dog>` key on an `Animal` is the error `E0235`.

An attribute declaration lists the fields of an attribute. In such a field, `static` is the class
that uses the attribute. For `#[Search(...)]` above `class Post`, `static` is `Post`. For an
attribute above a property, a method or a parameter, `static` is the class that contains it. A field
typed `property<static>` or `array<property<static>>` takes the names of that class's public
properties. The name of a `private` property, or of a property of another class, is `E0405` at the
string.

`Core\Attributes::get<Search>(Post::constructor)` reads the `Search` attribute of `Post`. It gives
`fields` the type `array<property<Post>>`, so `$post->$field` reads each property that `fields`
names.

`$key as string` gives the property name. Two keys are equal when they name the same property. A
key written in the code and a key converted from a variable are equal when the name is the same.
