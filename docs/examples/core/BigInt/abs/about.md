Returns a number without its sign, as a new `Core\BigInt`.

A negative number becomes positive. A positive number stays the same, and zero stays zero. The number
you called `abs` on does not change, because every `Core\BigInt` method returns a new number.

Use it when you want to know how far apart two numbers are. Subtract one from the other, then call
`abs` on the result. The order you subtract in then makes no difference to the answer.

**Good to know:** this works for every number, however large. `Core\Math::abs` on a plain `int`
throws an error for the smallest `int`, because the positive value is too large for an `int`. A
`Core\BigInt` has no such limit.
