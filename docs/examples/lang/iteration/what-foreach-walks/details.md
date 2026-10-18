The key of an array is always a `string`, also when the array was written as a list. `var $k` is a
`string`, and `int $k` is the error `E0723`. Convert the key in the loop body if you need a number.

A written value type is checked against the element type. `string $v` over an `array<int>` is the
error `E0401`. This is the same over an `Iterable<int>` and an `Iterator<int>`.

Any other subject is the error `E0443`. That covers an object whose class implements neither
interface, an `int`, a `string` and a `?array<int>`. Check a nullable array for `null` before the
loop.

A loop over an `Iterator` walks the iterator itself. A second loop over the same iterator starts
where the first one stopped. After a complete loop, the second loop gets no values. After a
`break`, the second loop gets the values that are left.
