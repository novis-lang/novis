Returns the hyperbolic cosine of a number. That is `(exp($n) + exp(-$n)) / 2`, where `exp` is
`Core\Math::exp`. This replaces PHP's `cosh`.

The result is never below `1.0`, and it is `1.0` at zero. A negative number gives the same result
as the positive one. The result grows very fast. Above about `710`, or below about `-710`, it is
too big for a `float` and becomes `INFINITY`. `NaN` (a value that means "not a number") gives
`NaN`.

**In plain words:** a cable or chain that hangs between two poles makes a curve. That curve is the
hyperbolic cosine.

**The examples below** show a few results, then very large numbers, then how far a cable between
two poles hangs down in the middle.
