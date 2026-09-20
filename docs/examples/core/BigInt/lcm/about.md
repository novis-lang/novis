Returns the least common multiple of this number and another one, as a new `Core\BigInt`.

The least common multiple is the smallest positive number that both of them divide exactly. Two
events that repeat every 4 and every 6 seconds happen together every 12 seconds, and 12 is what
`lcm` returns for 4 and 6.

The result is never negative, and the sign of the two numbers does not change it. The order of the
two numbers does not change it either. When one of them is zero the result is 0.

**Good to know:** the result can be far larger than either number. Two numbers that share no divisor
above 1 give their product, which is how a value grows past what an `int` holds.
