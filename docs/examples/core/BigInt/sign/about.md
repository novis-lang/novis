Tells you whether a `Core\BigInt` is below zero, at zero, or above zero.

`sign` returns -1 when the number is below zero, 0 when it is exactly zero, and 1 when it is above
zero. The answer is an `int`, so you can compare it and use it in an `if` straight away.

This is the same answer `Core\BigInt::compareTo` gives against zero. `sign` is the shorter way to
write it, because you need no second number.

A number far too large for an `int` has a sign like any other number. Reading the sign builds
nothing and looks at no digits, so it costs the same for a number of one digit and for a number of
a hundred thousand digits.
