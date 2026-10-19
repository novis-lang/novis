A method with `yield` in its body must return `Iterator<T>`. Any other return type is the error
`E0446`. This is true for `Iterable<T>`, `array<T>` and `void`. `Generator` is not declared, so it is
the error `E0303`. A method that returns `Iterator<T>` but has no `yield` is an ordinary method. It
must return a value, or it is the error `E0739`.

`yield` works only in the body of the generator itself. A `yield` at the top level of a file, or in
an anonymous function, is the error `E0445`. This is also true for an anonymous function inside a
generator.

A static method, an instance method and `iterate()` may each be a generator. An `iterate()` that
yields gives a new generator for each loop, so the object can be walked many times.

`yield` is a statement and has no value. These forms are the error `E0448`:

- `int $x = yield 2;`, since nothing is sent back into a generator.
- `yield "a" => 1;`, since an `Iterator<T>` has no keys.
- `yield;` with no value.
- `yield from G::inner();`. Write `foreach (G::inner() as int $v) { yield $v; }` instead.

The value of a `yield` must have the type `T`. `yield "a";` in an `Iterator<int>` is the error
`E0401`.

`return;` ends the sequence early. `return 5;` is the error `E0447`, because a generator has no
return value.

A generator cannot take an `inout` parameter. That is the error `E0492`. The arguments are copied
when the method is called. A change to the caller's array after the call is not seen by the
generator.

`current()` called before the first `advance()`, or after `advance()` returned `false`, throws a
`LogicError`. Once the sequence has ended, every later `advance()` returns `false`.

A loop over a generator cannot have a key. `foreach (G::g() as string $k => int $v)` is the error
`E0444`.

`clone G::g()` is the error `E0858`. `clone` of a generator in a variable throws a `LogicError` with
the message `clone(): a generator is a suspended frame, so it has no meaning on the other side of a
copy boundary`.

A `finally` block around a `yield` runs when the body reaches its end. It also runs when the loop
stops early with `break`, before the code after the loop. A generator that was advanced and then
freed runs its `finally` too. This happens when its last variable gets a new value. A generator that
was never advanced runs nothing.

A `throw` in the body reaches the `foreach` loop or the `advance()` call that resumed the generator.
The values yielded before it have already been given to the loop. A `try` and `catch` inside the body
can catch the error, and the body can then go on yielding.

A generator can read another generator with `foreach`, and a generator can call itself in the same
way. A generator stored in a `mixed` variable is an `Iterator<T>` for `is`. A `foreach` over the
`mixed` variable is the error `E0443`.
