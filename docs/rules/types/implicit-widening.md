Implicit conversion happens in exactly one place: an `int` or `uint` **widening into a `float`
position** — an argument, a return, an assignment, a field of an anonymous object, an element of an
array literal, or the far side of an arithmetic operator. A position whose type is a union holding
`float` is a `float` position for an `int` or `uint` value the union does not name, and a union value
such as a `?int` converts by its run-time tag: an integer tag becomes a `float`, and any other value
is stored as it is. A union that names `int`, such as `int|float`, keeps the `int`. A union source
whose integer members the target names only in part, such as `int|uint` into `int|float`, is not
converted. So `?float` and every other union holding `float` treat a whole number the way `float`
does, and adding `?` to a declared type never changes which values it accepts. It never reaches the
field of a shape value or object that already exists, because that value is shared and its field is not converted
(`rule:types/shape-type`). It never reaches the elements of an array that already exists either: they
keep the representation they were stored with, so an `array<int>` is not an `array<float>`, and
`as array<float>` is the conversion (`rule:types/arrays`). It throws above 2^53 rather than rounding,
where `f64` stops representing
every integer.

A branch of a ternary, an elvis or a `match` is not a `float` position. The checker types the whole
expression as the union of its branches, and branches of two representations join at that union's
tagged form, each tagged in its own block, instead of one being promoted into the other. So `$c ? 1 :
2.5` keeps PHP's answer on its truthy path, an `int` that stays exact past 2^53 where a widening
would have thrown, and `float $x = $c ? 1 : 2.5;` widens exactly once, at the binding. This is the
line integer `/` draws for the same reason: the promotion rows belong to an operator, whose result
type `rule:types/arithmetic` fixes, and a branch is neither an operator nor a position.

Everything else is a diagnostic. `mixed` never absorbs implicitly in either direction
(`rule:types/unions-and-mixed`), a `decimal` never meets a `float` in arithmetic
(`rule:types/decimal`), and every remaining change of type is written as `as`
(`rule:types/conversion`). A numeric literal is not a conversion at all: it is untyped until placed,
so it takes `int`, `uint`, `float` or `decimal` from its target (`rule:types/numeric-literal-placement`).

**Memory:** one span per widened value in the compile's expression table, and nothing per request.
A static integer at a union holding `float` pays the conversion a plain `float` position pays, and
a union value pays one tag test per store, at the widened positions only.

**Declined:**
- **Requiring `as ?float` for an `int` stored at a union holding `float`.** An `int` into `float`
  converts with nothing written, and so does an `int|float` value, so adding `?` to the declared type
  would change which values are accepted.
