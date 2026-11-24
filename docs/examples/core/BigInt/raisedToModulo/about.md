Raises a `Core\BigInt` to a power and gives the remainder, in one step.

`raisedToModulo` gives the same answer as raising the number to the power and then dividing by the
modulus, but the whole power is never built. The memory it uses stays about the size of the modulus,
however large the power is.

7 to the power 1000000 is a number of 845099 digits. `Core\BigInt::raisedTo` throws an error for
a number that wide. `raisedToModulo` gives its remainder at once.

The power may not be negative, and the modulus may not be 0. Both of those throw an
`ArithmeticError`.

This is the method used in a Diffie-Hellman key exchange and an RSA signature, where the numbers are
hundreds of digits long and every step is taken modulo a prime.
