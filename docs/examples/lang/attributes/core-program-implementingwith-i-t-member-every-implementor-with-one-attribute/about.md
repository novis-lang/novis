`Core\Program::implementingWith<I, T>($member)` gives you one row for every class in your program
that implements the interface `I`, with one of that class's attributes beside it.

Each row has two fields. `instance` is the object, and `attribute` is the one attribute on that
class's `$member` whose fields match the shape `T`. A class that carries no such attribute has `null`
there. `$member` is the name of a method or a property, or an empty string for the attributes written
on the class itself. The rows are sorted by the full class name, so the order is the same on every
machine.

Novis works all of this out while it compiles your program. This is how a framework reads the
settings of the classes it found: inside such a loop the variable is typed as the interface, so there
is no class name left to write into a lookup, and the compiler joins the attribute on for you.

**Good to know:** two attributes on one class that both match `T` are an error. Narrow the shape, or
read that class with `Core\Attributes::all<T>`.
