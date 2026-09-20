Shifts a `Core\BigInt` to the right.

`shr` moves every bit of the number `$bits` places to the right. The bits that fall off the right end
are gone. That is the same as dividing the number by two, `$bits` times, and rounding down.

Rounding down means rounding toward negative infinity, not toward zero. 7 shifted by 1 place is 3,
and -7 shifted by 1 place is -4.

You may shift by more places than the number is wide. A positive number then gives 0, and a negative
number gives -1. Shifting by 0 places gives the number back.

`shr` throws no error at all. The answer is never wider than the number you started with, so there
is no width to run into.

`shr` is a method because `>>` does not work on an object. To shift the other way, use
`Core\BigInt::shl`.
