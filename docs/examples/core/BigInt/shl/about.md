Shifts a `Core\BigInt` to the left.

`shl` moves every bit of the number `$bits` places to the left. That is the same as multiplying the
number by two, `$bits` times. 1 shifted by 10 places is 1024. Shifting by 0 places gives the number
back.

The sign stays what it was. A negative number stays negative, so -3 shifted by 4 places is -48.

Every place makes the number one bit wider. One call may produce a number of at most 1048576 bits,
and a wider answer throws an `ArithmeticError`. The width you start with counts toward that bound.
`shl` tests the width before it shifts, so the large number is never built and your program keeps
its memory.

`shl` is a method because `<<` does not work on an object. To shift the other way, use
`Core\BigInt::shr`.
