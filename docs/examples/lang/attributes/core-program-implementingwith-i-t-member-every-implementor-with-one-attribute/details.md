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
attributes, use `Core\Attributes::get` with `Report::constructor` and the parameter's name.

A name that one of the classes does not have is `E0798`, and the message names that class. A
method that a class inherits counts as one it has. Its attributes are not read, so that row's
`attribute` is `null`. A class that overrides a method has only the attributes on its own version.
An attribute on the interface's method is not read either.

`T` names an attribute declaration. The row's `attribute` is an attribute written with that name.
Another attribute with the same fields is not found. Two `T` attributes on one member of one class
are `E0728`, and the message names the class, the member and the count. A `T` that is not an
attribute declaration is `E1008`. That includes a shape such as `{ text: string }`, `int` and a
class.

The type of `attribute` is the declaration's fields, and it may be `null`. Write `var $row` in the
`foreach` to use this type. Storing the result in an array whose `attribute` cannot be `null` is
`E0401`.

`T` can also be an attribute that Novis reads itself, such as `Core\Route`. The row's `attribute` is
then the fields written in that attribute, such as its `path`.

When no class implements `I`, the result is `[]`, whatever `$member` is.
