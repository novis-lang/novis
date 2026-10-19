The element type of the result is the element type of the sequence. A generator of `int` gives an
`array<int>`, and a `Core\ObjectMap<string, int>` gives an `array<string>`. Assigning the result to
another array type is the error `E0401`.

An array literal written in the call gives an `array<mixed>`. `array<int> $b =
Core\Arr::from([1, 2, 3]);` is the error `E0401`. Put the array in a typed variable first.

The keys of an array are not kept. An array with the keys `"x"` and `"y"` gives an array with the
keys `0` and `1`.

The argument must be an array, an `Iterable<T>`, an `Iterator<T>` or a `Core` collection. Any other
type is the error `E0401`, and this is true for `mixed` too.

`limit` is a `uint`. A negative number or a string is the error `E0401`. `limit: 0` gives an empty
array, and the body of a generator does not run. A limit larger than the sequence gives the whole
sequence. Any other option name is the error `E0454`.

With `limit`, a generator stops where the limit is reached. A later loop or a later
`Core\Arr::from` over the same generator continues with the next value.

A heap is not emptied. `Core\Arr::from` gives its elements in priority order, and the heap still has
them all.

An error thrown by a generator reaches the code that called `Core\Arr::from`.
