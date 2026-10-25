A number written in the code without a type takes the type of the place it goes to. It can be an
`int`, a `uint`, a `float` or a `decimal`. `decimal $d = 3;` and `array<float> $xs = [1, 2];` both
compile.

An `int` or a `uint` above 9007199254740992 (2^53) throws an `ArithmeticError` when it is converted
to a `float`. This is also true for a number that a `float` could store exactly, such as
9007199254740994.

A single-value type is converted to its base type. A `"read"|"write"` value is converted to `string`,
and a `true` value to `bool`.

These assignments each need `as` or a test that narrows the type. Without it, each is the error
`E0401`:

- an `int` to a `uint`, and a `uint` to an `int`
- a number to a `string`, and a `string` to a number
- a `float` to an `int`
- a `mixed` value to any other type
- a `?int` to an `int`. The help suggests `$value ?? 0`.

An anonymous object written at a shape type has its fields converted too. `{ x: 2 }` at `{ x: float }`
stores `2.0`. A shape value that already exists is not converted. A `{ x: int }` value at
`{ x: float }` is the error `E0401`. Build a new value instead: `{ x: $old->x }`.
