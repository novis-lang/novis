A numeric literal is **untyped until it is placed**, and takes its type from the position it appears
in. An integer literal becomes `int`, `uint`, `float` or `decimal`; a literal carrying a fractional
part or an exponent becomes `decimal` or `float`. Because every binding site declares a type
(`rule:types/declaration`), the target is known almost everywhere.

```nvs
decimal $price = 19.99;          // exact: mantissa 1999, scale 2
float   $ratio = 19.99;          // an f64
var $x = 19.99;                  // no target type: float
var $y = 19.99 as decimal;       // `as` supplies one: decimal, exact
```

**A union holding `decimal` is a `decimal` position for a literal its other members do not accept.**
`?decimal $rate = 3;`, `decimal|string $label = 7.25;` and the `?decimal` field of an anonymous object
place the literal at `decimal`, exact, at every position a plain `decimal` does. A union that already
accepts the literal's own type keeps it: `int|decimal` places `3` at `int`, and `float|decimal` places
`3` and `1.5` at `float`. Only a literal is placed; an `int` variable at `?decimal` is still a mismatch.

**`expr as T` is itself a placing position.** A literal written directly under a conversion takes `T`
as its target rather than being typed first and converted afterwards, so `19.99 as decimal` is exact
to the full 29 significant digits and never becomes an `f64` on the way. That is not merely notational:
`float → decimal` recovers only the ~17 digits an `f64` round-trips, so without this rule a wider
literal would be unwritable in any position lacking an annotation.

**There is no literal suffix, and in particular no `m`** (`rule:types/integer-literals`). A suffix
would buy only a second spelling of what `as decimal` already says, in the two positions that lack a
target: a `var` declaration (`rule:types/var-inference`) and a `mixed` or generic argument. C# needs
its `m` only because it refuses `decimal d = 19.99;`, a fractional literal there being born a `double`;
placement removes that constraint.

**Declined:**
- **An `m` suffix, `19.99m`.** It costs a decimal-specific lexer production and its own diagnostics (a
  suffix in a `float` position, a suffix on a hex literal) to spell in two positions what `as decimal`
  spells everywhere, and two ways to write one value is the duplication `rule:core-api/shape-rules`
  refuses. A trailing `m` lexes as a float literal followed by a stray identifier, which does not parse.
- **A `d` suffix.** In C# and Java it already means *double*, so `19.99d` would read as the wrong thing.
