Compares two `bytes` values and tells you which of them comes first in order.

The result is `-1` when the first value comes first, `0` when the two are the same, and `1` when the
second value comes first. The bytes are compared one at a time from the start, each as a number from
0 to 255. When one value is the start of the other, the shorter one comes first. Use this to sort
binary keys, or to test whether a key falls between two boundaries.

**Good to know:** to test only whether two values are the same, write `$a == $b`. That is shorter to
read, and it stops at the first byte that differs.
