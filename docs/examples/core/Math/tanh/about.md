Returns the hyperbolic tangent of a number. That is `Core\Math::sinh` divided by `Core\Math::cosh`.

The result is always from `-1.0` to `1.0`, and it has the same sign as the number. It is `0.0` at
zero. For large numbers it gets very close to `1.0`, and from about `20` it is exactly `1.0`.
`INFINITY` gives `1.0` and `-INFINITY` gives `-1.0`. `NaN` (a value that means "not a number")
gives `NaN`.

**In plain words:** `Core\Math::tanh` turns any number into a number from `-1.0` to `1.0`. A number
close to zero changes very little. A large number ends up close to `1.0` or `-1.0`.

**The examples below** show a few results, then large numbers, then how a website turns votes of
any size into a score from -1 to 1.
