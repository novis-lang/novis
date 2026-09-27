Returns the declared type of one parameter of a method, as text.

`type()` returns the type written the same way the method writes it: `int`, `?int` for an `int`
that may be `null`, `array<string>`, or the name of a class. A parameter declared in a source file
always has a type, so the result is text. The result is `null` only if no type is known.

The result is the text of the declaration. It is not a value you can check a value against. To
check the type of a value, use `Core\Reflect::typeOf`.

**The examples below** print the type of each parameter, show nullable, array and class types,
and write the help text for a web API from the parameters of its handler.
