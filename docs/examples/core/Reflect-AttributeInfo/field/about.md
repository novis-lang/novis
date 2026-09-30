Returns the value of one field of an attribute, by the name of the field.

The value is a `string`, an `int`, a `bool` or a `float`. The return type is `mixed`, so you convert
the result with `as` to the type you expect.

`field` throws a `LogicError` when the attribute has no field with that name. `fields` returns the
names that exist.

**Good to know:** `field` reads a value that is written directly as a text, a number, `true` or
`false`. It throws a `LogicError` when the value is a constant, an enum case or the name of a
class. `Core\Attributes::get` reads those values too.
