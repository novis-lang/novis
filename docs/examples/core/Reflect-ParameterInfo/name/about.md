Returns the name of one parameter of a method.

`name()` returns the name that the method declares, without the `$`. For
`send(string $to)`, the name is `to`. This is also the name you write for a named argument.

A constructor parameter that is also a property, such as `constructor(private string $from)`, has
the same name as the property.

You get a `Core\Reflect\ParameterInfo` from `Core\Reflect\MethodInfo::parameters`, which lists the
parameters in the order they are written.

**The examples below** print the parameters of a method with a `$`, show a constructor parameter
that is also a property, and call a method with values read from a configuration by name.
