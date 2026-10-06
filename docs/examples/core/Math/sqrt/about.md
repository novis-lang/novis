`Core\Math::sqrt` returns the square root of a number. The square root of `9.0` is `3.0`, because
`3.0 * 3.0` is `9.0`.

The result is a `float`. A negative number has no square root, so the result is `NaN` (a value that
means "not a number"). `Core\Math::sqrt` does not throw an error for it. Use `Core\Math::isNan` to
check the result. `-0.0` gives `-0.0`, and `Core\Math::INFINITY` gives `Core\Math::INFINITY`.

**The examples below** show the square roots of simple numbers, then negative numbers and special
values, then how much a list of response times varies.
