Returns a random whole number between two numbers, and both numbers can be returned.

`Core\Random::int(1, 6)` rolls a die. It returns 1, 2, 3, 4, 5 or 6, and each one comes up
equally often. When the two numbers are the same, the result is that number. The number comes from
a cryptographic generator, so it is safe to use for a login code or a PIN.

The first number must not be larger than the second. If it is, the method throws an error. It does
not swap the two numbers for you, because wrong bounds usually mean a bug in the calculation
before the call.
