Returns the number whose hyperbolic cosine is the number you give it. This is the inverse
hyperbolic cosine, and it undoes `Core\Math::cosh`.

The number must be `1.0` or more, because a hyperbolic cosine is never smaller than `1.0`. The
result is `0.0` or more. A number below `1.0` gives `NaN` (a value that means "not a number"), and
`Core\Math::isNan` tests for it. This replaces PHP's `acosh`.

**In plain words:** a cable that hangs between two poles has the shape of a `cosh` curve. `acosh`
works the other way. From the height of a point on the cable, it finds how far that point is from
the lowest point.

**The examples below** show a few results, then what happens below `1.0`, then how far apart two
poles can stand when a cable may sag a given amount.
