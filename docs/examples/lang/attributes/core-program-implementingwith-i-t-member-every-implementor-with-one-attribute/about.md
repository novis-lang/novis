`Core\Program::implementingWith<I, T>($member)` gives you one row for every class in your program
that implements the interface `I`, with one of that class's attributes beside it.

`T` is the name of an attribute declaration, such as `Menu` after `attribute Menu on method { … }`.
Each row has two fields. `instance` is the object, and `attribute` is the `T` attribute on that
class's `$member`. Without one, it is `null`. `$member` is the name of a method or a property, or an
empty string for the attributes written on the class itself. Rows are sorted by full class name.

Novis finds the classes and their attributes while it compiles your program. A framework uses this
to read the settings of the classes it found.

**Good to know:** a class with two `T` attributes on one member is an error. Read that class with
`Core\Attributes::all<T>`.

The examples show a report label, class attributes, an admin menu, two attributes with the same
field, and a missing member, which does not compile.
