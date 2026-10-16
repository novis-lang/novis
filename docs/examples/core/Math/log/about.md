`Core\Math::log` returns the logarithm of a number. The logarithm is the power you raise the base
to, to get that number. In base `10`, the logarithm of `1000.0` is `3.0`,
because `10 ** 3` is `1000`.

The base is `E` (about `2.718`) when you leave it out. You choose another base with the `base`
option, for example `{base: 10.0}` or `{base: 2.0}`. In base `10.0` and base `2.0`, the result for
a power of that base is exact.

The logarithm of `0.0` is `-INFINITY`. A negative number has no logarithm, so the result is `NaN`
(a value that means "not a number"). A base of zero or below throws a `RuntimeError`.

**The examples below** show the three common bases, then the numbers with no logarithm, then how
long it takes for savings to double.
