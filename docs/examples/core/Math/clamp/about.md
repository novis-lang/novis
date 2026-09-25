Keeps a value between a lowest and a highest value. A value below the range becomes the lowest
value, and a value above it becomes the highest. A value inside the range stays the same, and both
ends count as inside. This replaces PHP's `min(max($n, $low), $high)`.

It works for numbers, strings, `bool` and `null`, as long as all three values can be compared with
each other. When the lowest value is above the highest, there is no range, and `Core\Math::clamp`
throws a `RuntimeError`.

Use it when a number comes from outside your program and has to stay within limits: a page size a
visitor asks for, a volume, a percentage.

**The examples below** show whole numbers kept in a range, then a range with no values in it, then
a page size limited to what a server allows.
