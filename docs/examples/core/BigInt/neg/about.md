Returns this number with its sign flipped, as a new `Core\BigInt`.

A positive number becomes negative, and a negative number becomes positive. Zero stays zero. The
number you called `neg` on does not change, because every `Core\BigInt` method returns a new number.

`Core\BigInt` has no operators, so `neg` is how you write what `-$value` writes for a plain `int`.
Calling it twice gives the number you started with. Adding a number's `neg` gives the same result as
subtracting that number, which is useful when a program keeps one list of amounts and books some of
them in the other direction.

**Good to know:** this works for every number, however large. Flipping the sign of the smallest
`int` throws an error, because the positive value is too large for an `int`. A `Core\BigInt` has no
such limit.
