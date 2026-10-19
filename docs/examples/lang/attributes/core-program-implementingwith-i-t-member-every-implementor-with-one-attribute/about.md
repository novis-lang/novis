`Core\Program::implementingWith<I, T>($member)` gives you one row for every class in your program
that implements the interface `I`, with one of that class's attributes beside it.

Each row has two fields. `instance` is the object, and `attribute` is the one attribute on that
class's `$member` whose fields match the shape `T`. A class that carries no such attribute has `null`
there. `$member` is the name of a method or a property, or an empty string for the attributes written
on the class itself. The rows are sorted by the full class name, so the order is the same on every
machine.

Novis finds the classes and their attributes while it compiles your program. A framework uses this
to read the settings of the classes it found. In a loop over `Core\Program::implementing`, the
variable has the type of the interface. `Core\Attributes::get` needs a class name, so it cannot read
the attributes there. `implementingWith` reads them for each class.

**Good to know:** two attributes on one class that both match `T` are an error. Use a narrower
shape, or read that class with `Core\Attributes::all<T>`.

The examples show a report label, the attributes on a class itself, an admin menu, a class without
the member, which does not compile, and the path of each page's route.
