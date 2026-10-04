A `callable` type may name its parameters and its return type:

```php
callable(User, string): string   $format;
callable(): void                 $onExit;
callable                         $anything;
```

The **return type is mandatory** — `callable(int)` with no return says strictly less than bare
`callable` while costing a second spelling of "unknown". `void` and `never` are writable there as in
any other return position. **No parameter names**: `callable(int $x): string` does not parse, because
a name in the type would imply calling through the value by name, which nothing supports.

Bare `callable` remains the **top of the callable lattice** — a callable whose signature is unknown.
Every callable type is assignable to it, calling through one keeps the dynamic path and its
per-argument tag check, and nothing existing changes meaning. Narrowing is opt-in. Where a call
reaches a callable whose type names its parameters, the arguments are proven at compile time and the
per-argument tag check is **not emitted**; the metadata stays on every callable, because bare `callable`
still needs it and an anonymous function does not know where it is written which kind of site will call it.

The two binding-site variants that stood in for this — a callback-return parameter and a shape of
callbacks — are retired, and the restriction that a shape's every field be a *written* anonymous function
goes with them, because a `callable(): T`-typed variable now carries what the field needs.
