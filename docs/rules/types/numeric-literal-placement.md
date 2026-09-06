A numeric literal is **untyped until it is placed**, and takes its type from the position it appears
in. An integer literal becomes `int`, `uint`, `float` or `decimal`; a literal carrying a fractional
part or an exponent becomes `decimal` or `float`. Because every binding site declares a type
(`rule:types/declaration`), the target is known almost everywhere.

```php
decimal $price = 19.99;          // exact: mantissa 1999, scale 2
float   $ratio = 19.99;          // an f64
var $x = 19.99;                  // no target type: float
var $y = 19.99 as decimal;       // `as` supplies one: decimal, exact
```

**`expr as T` is itself a placing position.** A literal written directly under a conversion takes `T`
as its target rather than being typed first and converted afterwards, so `19.99 as decimal` is exact
to the full 29 significant digits and never becomes an `f64` on the way. That is not merely notational:
`float → decimal` recovers only the ~17 digits an `f64` round-trips, so without this rule a wider
literal would be unwritable in any position lacking an annotation.

**There is no literal suffix, and in particular no `m`** (`rule:types/integer-literals`). A suffix
would buy only a second spelling of what `as decimal` already says, in the two positions that lack a
target: a `var` declaration (`rule:types/var-inference`) and a `mixed` or generic argument.
