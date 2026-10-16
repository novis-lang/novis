Compares this number with another one and returns `-1`, `0` or `1`.

The result is `-1` when this number is the smaller of the two, `0` when they are equal, and `1` when
this number is the larger. Numbers of any size are compared exactly.

You can also write `$a < $b`, `$a > $b`, `$a <= $b`, `$a >= $b` and `$a <=> $b` on two `Core\BigInt`
values, and `Core\Arr::sort` puts a list of them in order for you. All of those use `compareTo`, so
they always agree with it. Reach for `compareTo` itself when you want the three-way answer.

**Good to know:** a negative number is always the smaller one, however many digits it has.
