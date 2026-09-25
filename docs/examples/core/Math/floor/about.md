Rounds a number down to the next whole number. `4.9` becomes `4.0`, and a number that is already
whole stays the same.

For a negative number, down means away from zero: `-4.1` becomes `-5.0`. A number between `-1.0`
and `0.0` becomes `-1.0`. The result is always a `float`. `NaN` (a value that means "not a
number") and the infinities stay the same. This replaces PHP's `floor`.

Use it when only a complete unit counts: a full ten euros, a full hour, a full box.
`Core\Math::ceil` rounds up, `Core\Math::truncate` rounds toward zero, and `Core\Math::round`
rounds to the nearest number.

**The examples below** show a few numbers rounded down, then negative numbers, then how a shop
gives one loyalty point for every full ten euros a customer spends.
