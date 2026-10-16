Returns the declared type of one property of a class, as text.

`type()` returns the type written the same way the class writes it: `int`, `?int` for an `int` that
may be `null`, `array<string>`, or the name of a class. A property declared in a source file always
has a type, so the result is text. The properties of the built-in error classes, such as `message`
on `LogicError`, have no written type, so for them the result is `null`.

The result is the text of the declaration. It is not a value you can check a value against. To
check the type of a value, use `Core\Reflect::typeOf`.

**The examples below** print the type of each property, find the properties that may be `null`,
and convert the text of a web form to the type each property needs.
