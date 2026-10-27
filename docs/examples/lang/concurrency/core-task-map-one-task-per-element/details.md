The function may take no parameters, the value, or the value and the key. A third parameter is
`E0401`. The key is always a `string`, so `fn(int $v, int $k)` is `E0401`. The value parameter
must have the type of the array's elements.

The result is an array of the function's return type. `Core\Task::map([1, 2], fn(int $v): int => …)`
returns an `array<int>`.

The function can be written in place, or be a method reference such as `Prices::of`. A
variable of type `callable(int): int` works too. A variable of type `callable`, with no signature,
is `E0401`.

The first argument must be an array. An `Iterator` or a `mixed` value is `E0401`.

An empty array starts no tasks, and the result is an empty array.
