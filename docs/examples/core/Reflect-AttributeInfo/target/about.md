Returns the name of the property or method that an attribute is written on.

An attribute can be written on a class, on a property, on a method or on a parameter of a method.
For a property, `target` returns its name without the `$`. For a method, it returns the name of the
method. For an attribute on the class or interface itself, it returns an empty string.

An attribute on a parameter has the name of its method as `target`. The `parameter` method returns
the name of the parameter.

Use `target` to find the attributes of one property, or to connect each property with a setting such
as the name of a database column.
