Implicit conversion happens in exactly one place: an `int` or `uint` **widening into a `float`
position** — an argument, a return, an assignment, a field of an object literal, or the far side of
an arithmetic operator. It never reaches the field of a shape value or object that already exists,
because that value is shared and its field is not converted (`rule:types/shape-type`). It is
the one coercion PHP's own `strict_types` permits, and it throws above 2^53 rather than rounding,
where `f64` stops representing every integer.

Everything else is a diagnostic. `mixed` never absorbs implicitly in either direction
(`rule:types/unions-and-mixed`), a `decimal` never meets a `float` in arithmetic
(`rule:types/decimal`), and every remaining change of type is written as `as`
(`rule:types/conversion`). A numeric literal is not a conversion at all: it is untyped until placed,
so it takes `int`, `uint`, `float` or `decimal` from its target (`rule:types/numeric-literal-placement`).
