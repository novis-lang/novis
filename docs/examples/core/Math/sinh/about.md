Returns the hyperbolic sine of a number. That is `(exp($n) - exp(-$n)) / 2`,
where `exp` is `Core\Math::exp`.

The result is `0.0` at zero, and it has the same sign as the number. A negative number gives the
same result as the positive one, with a minus sign. The result grows very fast. Above about `710`
it is too big for a `float` and becomes `INFINITY`. Below about `-710` it becomes `-INFINITY`.
`NaN` (a value that means "not a number") gives `NaN`.

**In plain words:** a cable that hangs between two poles makes a curve. The hyperbolic sine gives
the length of that curve.

**The examples below** show a few results, then very large numbers, then how much cable is needed
between two poles.
