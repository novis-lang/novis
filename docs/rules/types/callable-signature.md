A `callable` type may name its parameters and its return type:

```nvs
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

**The reason is safety, not convenience.** A compiled callable reads each argument at its own declared
representation, so a mismatched argument is reinterpreted: an `int` read as a string pointer is an
arbitrary dereference, not a fault. A bare `callable` guards that with a runtime tag check per
argument, per call. A written signature discharges the same guard at compile time, as an error where
the mismatch is written. `Core\Arr::map($ints, fn (string $s): string => $s)` over an `array<int>`
does not compile.

The parameter list needs no trial parse: a `(` after `callable` in a type position can only open a
parameter list, because a type position has no call syntax to confuse it with.

The binding-site variant that stood in for a callback's return type is retired: every `Core`
signature that used it writes an ordinary `callable(T, string): U` instead, and a type variable binds
by walking into it the way it walks into `array<T>`. The restriction that a shape's every field be a
*written* anonymous function goes too, because a `callable(): T`-typed variable now carries what the
field needs. The shape-of-callables type that `Core\Task::all` takes stays, as a bound and nothing
more: no type variable can be bounded, and without it `Task::all(5)` would compile and answer `mixed`.
What it accepts is ordinary assignability, and what it binds is the same structural walk.

- **Memory:** none. A written signature adds no runtime representation, no heap shape and no
  per-callable growth. The parameter tags and the arity stay on every callable value.

**Declined:**
- **Angle brackets, `callable<User, string, string>`.** The return type would be positional and
  unmarked, and `callable<void>` reads as taking a `void`. Seeing the return type is worth more than
  matching the bracket shape of `array<T>`.
- **An optional return type.** `callable(int)` would be a third level of partial knowledge that still
  cannot bind a result type variable, so it buys argument checking at the price of a second syntax.
- **Optional and variadic parameters in a callable type.** `callable(int, ?string = null): void` and
  `callable(int, ...string): void` do not parse. The prefix rule (`rule:types/callable-arity`)
  already covers a callback that ignores trailing arguments, and no `Core` callback's own arity
  varies.
- **`Closure` as a second name for a typed signature.** One function type is enough
  (`rule:types/callable-is-the-only-function-type`).
