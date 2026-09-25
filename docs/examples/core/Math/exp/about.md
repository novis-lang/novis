Returns `E` raised to the power of the number you give it. `E` is a fixed number, about `2.718`,
and `Core\Math::E` contains it. So `Core\Math::exp(1.0)` is `E`, and `Core\Math::exp(0.0)` is
`1.0`. This replaces PHP's `exp`.

The result is never negative, and it grows very fast. Above about `709.78` it is too big for a
`float`, and the result is `INFINITY`. Far enough below zero, the result is `0.0`.
`Core\Math::log` does the reverse. `NaN` (a value that means "not a number") gives `NaN`.

**In plain words:** `exp` describes something that grows or shrinks by the same share of its size
all the time. Money with interest grows this way. A score that loses value with age shrinks this
way.

**The examples below** show a few results, then numbers that are too big or too small, then how a
website ranks posts so that a new post can come before an older one with more votes.
