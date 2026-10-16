Lists the constants of a class.

`constants()` returns an array with one `Core\Reflect\ConstantInfo` for each constant of the class.
The class's own constants come first. Then come the constants it inherits from parent classes and
interfaces. When a subclass declares a constant again, the list has it once, with the subclass's
declaration.

Each item has the constant's name and says whether it is public. It also says whether the
constant has a value that `constant` can return. An `array` constant has no such value. Private
and protected constants are in the list too. The list does not contain the values: use
`constant($name)` to read one.

**The examples below** show the list for one class, the list for a subclass, and a report that
prints every public limit of a class with its value.
