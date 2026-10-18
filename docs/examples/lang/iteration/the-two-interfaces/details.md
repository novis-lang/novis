`implements Iterator` with no type argument is the error `E0442`. Write the element type, such as
`Iterator<int>`. A class of your own cannot declare a type argument. `class Box<T>` and
`interface Pair<T>` do not compile.

A class that implements `Iterator<T>` without `advance()` or `current()` is the error `E0449`. The
same is true for `Iterable<T>` without `iterate()`.

A class may implement both interfaces. `iterate()` may then return `$this`.

The three types are separate. An `array<int>` and an `Iterator<int>` are not an `Iterable<int>`, and
an `Iterable<int>` object is not an `Iterator<int>`. Passing one where another is needed is the error
`E0401`. The element type must match too: an `Iterable<int>` object is not an
`Iterable<int|string>`.

`current()` called before the first `advance()`, or after `advance()` returned `false`, throws a
`LogicError`. The message is `current() outside the iteration protocol: it answers only after
advance() returned true`.
