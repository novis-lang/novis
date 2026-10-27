These forms do not compile:

- `function (int $x) { … }` is the error `E0222`.
- `fn(int $x) use ($k) => …` is the error `E0223`.
- `static fn(…) => …` is the error `E0210`.
- An `inout` parameter is the error `E0493`, and `int &$x` is the error `E0237`.
- A parameter without a type is the error `E0808`.
- A block body without a return type is the error `E0450`.

With an expression body, the return type may be left out. The function then returns the type of the
expression. A value of the wrong type is the error `E0401` in an expression body, and the error
`E0403` after `return` in a block.

A captured variable is a copy. The function may change its copy, and the outside variable keeps its
value. Each call starts again from the value that was captured: a function that adds 1 to its copy
of `$k = 1` returns `2` on every call. An object is not copied. A function that captured `$b` reads
a property of `$b` that changed after the function was written.

A function cannot use the variable it is stored in. In `callable $f = fn(…) => … $f(…) …`, the inner
`$f` is the error `E0301`, because `$f` has no value yet. Give the function a name instead:
`fn fact(int $n): int => …` can call `fact(…)`. The call is checked against the function's
parameters and return type. Outside the function, `fact` is the error `E0320`.

`$this` outside a method, or inside a `static` method, is the error `E0779`.

A method reference names a method without calling it. `Core\Str::length`, `self::f` and
`static::f` each give a callable. On an object, write `(...)` in place of the arguments:
`$p->add(...)`. A method that does not exist is the error `E0405`.

`bindTo`, `bind` and `call` can throw a `LogicError` for the wrong object. Its message starts
``this callable uses `$this`, so it can only be bound to an object of class `A` ``. A function that does not use `$this` is returned unchanged, so any object works. A second
argument is the error `E0402`.

An anonymous function can be called where it is written: `(fn(int $x): int => $x * 10)(4)` gives
`40`.
