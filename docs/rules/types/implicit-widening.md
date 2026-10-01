Implicit conversion happens in exactly one place: an `int` or `uint` **widening into a `float`
position** — an argument, a return, an assignment, a field of an object literal, an element of an
array literal, or the far side of an arithmetic operator. A position whose type is a union holding
`float` is a `float` position for an `int` or `uint` value the union does not name, and a union value
such as a `?int` converts by its run-time tag. It never reaches the field of a shape value
or object that already exists, because that value is shared and its field is not converted
(`rule:types/shape-type`). It never reaches the elements of an array that already exists either: they
keep the representation they were stored with, so an `array<int>` is not an `array<float>`, and
`as array<float>` is the conversion (`rule:types/arrays`). It is the one coercion PHP's own
`strict_types` permits, and it throws above 2^53 rather than rounding, where `f64` stops representing
every integer.

Everything else is a diagnostic. `mixed` never absorbs implicitly in either direction
(`rule:types/unions-and-mixed`), a `decimal` never meets a `float` in arithmetic
(`rule:types/decimal`), and every remaining change of type is written as `as`
(`rule:types/conversion`). A numeric literal is not a conversion at all: it is untyped until placed,
so it takes `int`, `uint`, `float` or `decimal` from its target (`rule:types/numeric-literal-placement`).
