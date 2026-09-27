Returns a list with one description for each property of a class.

`properties()` returns an array of `Core\Reflect\PropertyInfo`. Each one has the name of a property,
its declared type, and whether it is public. The list has every property the class declares, and
private and protected properties are in it too. The order is the order of the declaration, and
properties from a parent class come first. It replaces PHP's `ReflectionClass::getProperties`.

The list only describes the properties. It does not read their values. To read a value, use `get`,
which checks visibility the same way a normal read does. `readableProperties` returns only the
names that your code can read at the place where it calls it.

**The examples below** list the properties of a class, show that a child class lists the properties
of its parent first, and build the header line of a table.
