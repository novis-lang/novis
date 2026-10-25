`I` follows the same rules as for `Core\Program::implementing`. The list has the same classes in
the same order, and `E0743`, `E0744` and `E0442` are given for the same mistakes. The call takes two
type arguments.

`$member` is a `string`, and an `int` is `E0401`. Without an argument, `$member` is the empty
string, so the rows have the attributes on each class itself. A second argument is `E0402`.

Write `$member` as a string in quotes. Novis reads it while it compiles. A variable, a class
constant or a joined string such as `"m" . "m"` also compiles, but every row's `attribute` is
then `null`.

A method name finds the method in each class, also a `static` or a `private` one. `"constructor"`
finds the attributes on the constructor. A property is written without the `$`, as `"total"`. In
double quotes, `"$total"` is a variable in the string, so it is `E0301` when no such variable
exists. A promoted constructor parameter is a property, and its name gives the attributes written
on that parameter. A constructor parameter that is not promoted is not found. To read its
attributes, use `Core\Attributes::get` with `Report::constructor(...)` and the parameter's name.

A name that one of the classes does not have is `E0798`, and the message names that class. A
method that a class inherits counts as one it has. Its attributes are not read, so that row's
`attribute` is `null`. A class that overrides a method has only the attributes on its own version.
An attribute on the interface's method is not read either.

The attribute matches `T` when it has every field of `T` with the right type. It may have more
fields. Two attributes that both match on one class are `E0728`, and the message names the class and
the member. A shape with a field that only one of them has picks that one. A `T` that is not a shape,
such as `int` or a class, is `E0729`.

Each row has the type `{ instance: I, attribute: ?T }`. Storing the result in an
`array<{ instance: I, attribute: T }>` is `E0401`, because `attribute` may be `null`.

The attributes Novis reads itself can be read this way too. `{ path: string }` reads the path of each
class's `#[Core\Route]`.

When no class implements `I`, the result is `[]`, whatever `$member` is.
