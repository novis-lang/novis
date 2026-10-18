A property path is a value that names a field across related objects, such as `customer.address.city`.

The type `propertyPath<Order>` contains paths that start at the class `Order`. Each name in a path is a
public property. The first name is a property of `Order`. Each name after it is a property of the
class that the name before it reaches. That property can be an object, a nullable object or an array
of objects. A string written in a call to a `propertyPath<Order>` parameter is checked when the
program compiles. A wrong name does not compile, and the error points at that name. `as` checks any
other string while the program runs. If a name is wrong, `as` throws an error.
`as ?propertyPath<Order>` gives `null` instead.

**Good to know:** a `private` or `protected` property is never part of a path. A path can come back to
the same class, such as `parent.parent.name`, and has no length limit. `$path as string` gives the
text of the path.

**The examples below** go from an order to its customer, address and lines, sort a list by a field
from the web address, and use `propertyPath<static>` in a parent class. Then they show the errors for
a wrong path in a string and in the code.
