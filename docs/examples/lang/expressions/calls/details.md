The arguments run in the order they are written, also when they are named. In
`A::f(b: A::v(2), a: A::v(5))`, `A::v(2)` runs first.

A static method may also be called on an object: `$p->s()` calls the static method `s`. An
instance method called on the class, such as `P::i()`, is the error `E0778`. A method the class does
not have is the error `E0405`. On a nullable object, such as a `?P`, `$p->hi()` is the error `E0459`.
Use `$p?->hi()`, which gives `null` when `$p` is `null`.

`...$list` only fills a variadic parameter. Before a method with only fixed parameters, it is the
error `E0489`. Arguments may come before it, and several `...` arguments may follow each other:
`A::f(3, ...$xs)` and `A::f(...$xs, ...$xs)` both work. The type of the array must match the
variadic parameter, so an `array<string>` for `int ...$a` is the error `E0401`. The keys of the
array are kept: `["x" => 1, "y" => 2]` arrives with the keys `"x"` and `"y"`.

A call through a `callable` gives `mixed`. `int $r = $f(1)` is the error `E0401`, so write
`$f(1) as int`. The arguments are checked when the call runs. An argument of the wrong type throws a
`LogicError`: ``argument 1 to a `callable` must be of type int, string given``. Too few arguments
throw a `LogicError` too, and extra arguments are ignored.

A `callable` in a property needs brackets around it: `($this->cb)(1)`. Without them,
`$this->cb(1)` calls a method named `cb`, and is the error `E0405` when the class has none.

A call that returns `void` has no value. At an `int` variable it is the error `E0401`, and after
`echo` it is the error `E0707`.
