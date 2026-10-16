Returns the name of the method parameter that an attribute is written on.

The name has no `$`. For an attribute on a class, a property or a method, the result is an empty
string.

Use `parameter` together with `target`. `target` gives the method, and `parameter` gives the
parameter of that method. An attribute on a method and an attribute on one of its parameters have
the same `target`. Only the second one has a `parameter` that is not empty.

A web framework can use this to find out where each argument of a handler method comes from, for
example from the path or from the request body.
