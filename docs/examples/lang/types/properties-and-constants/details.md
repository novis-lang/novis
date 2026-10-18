A property needs a type. `public $x = 1;` is a syntax error (`E0101`).

A default must be a constant of the property's type. That is a value written in the code, also with
a minus sign such as `-3`. It can also be `[]`, an enum case, or a class constant such as
`Limits::ATTEMPTS` or `self::ATTEMPTS`. A union type takes any value one of its members takes:
`?string $label = null`, `?string $label = "plain"` and `"read"|"write" $mode = "read"` all compile.
Any other default does not compile (`E0472`). That includes a call, `null` for a `string` property,
and `"delete"` for a `"read"|"write"` property.

A property with no default must be set on every path out of the constructor. A constructor that sets
it only inside an `if` does not compile (`E0409`). A `static` property has no constructor to set it.
It needs a default or a nullable type, and a nullable one starts as `null`. Without either, it is
`E0409` too.

A constant needs a type. `public const W = 5;` does not compile (`E0246`). Reading a constant gives
its declared type exactly. With `public const uint WIDTH = 5;`, `int $i = Limits::WIDTH;` does not
compile (`E0401`).

The compiler copies a constant's value into every place that reads it. So the value must be a
number, a string, a `bool`, or an array of them. A constant whose value is another class's
constant, an enum case, `self::B + 1` or `"a" . "b"` compiles, but every read of it does not
compile (`E0792`).
