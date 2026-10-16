Returns the number whose hyperbolic tangent is the number you give it. This is the inverse
hyperbolic tangent, and it undoes `Core\Math::tanh`.

The number must be from `-1.0` to `1.0`, because a hyperbolic tangent is always in that range.
`1.0` gives `INF` (infinity), and `-1.0` gives `-INF`. A number outside the range gives `NaN` (a
value that means "not a number"), and `Core\Math::isNan` tests for it.

**In plain words:** `tanh` squeezes any number into the range from `-1.0` to `1.0`. `atanh` stretches
a number from that range back out. Numbers close to `1.0` or `-1.0` become very big.

**The examples below** show a few results, then what happens at the ends of the range and outside
it, then how a report finds the average of several correlations.
