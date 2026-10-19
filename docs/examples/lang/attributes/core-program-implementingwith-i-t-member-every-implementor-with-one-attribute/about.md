`Core\Program::implementingWith<I, T>($member)` gives you one row for every class in your program
that implements the interface `I`, with one of that class's attributes beside it.

Each row has two fields. `instance` is the object, and `attribute` is the one attribute on that
class's `$member` whose fields match the shape `T`. Without such an attribute, it is `null`. `$member` is the name of a method or a property, or an empty string for the attributes written
on the class itself. Rows are sorted by full class name.

Novis finds the classes and their attributes while it compiles your program. A framework uses this
to read the settings of the classes it found.

**Good to know:** two attributes on one class that both match `T` are an error. Use a narrower
shape, or read that class with `Core\Attributes::all<T>`.

The examples show a report label, class attributes, an admin menu, each page's route path, and a
missing member, which does not compile.
