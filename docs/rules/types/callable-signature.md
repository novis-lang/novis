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

**A parameter that names a class is checked against the argument's ancestry at the callable's
entry.** The tag check compares representations, and every class name, `object` and a shape share one
tag, so it accepts an instance of an unrelated class. A named-class binding is read and written at a
fixed offset against its label, so that acceptance is a type confusion and not a wrong answer: one
wrong callback can write an `int` over a `string` field, and the next read dereferences it. That is
security and request isolation. Every other position is checked where it is written, and an erased
receiver carries no label at all (`rule:types/erased-member-access`), so the callable's entry is the
one place nothing else looked. The body's first block runs the same class test `$x is C` runs on each
class-declared parameter, and a miss throws a `LogicError`. A parent class and an implemented
interface accept the instance an exact class does.

Two things the entry check does not do. A `?C` parameter is unchecked on both lines: it erases to a
tagged value before any class survives, the same nothing `mixed` gets. And the error does not name the
class that arrived, because no compiled instruction reads an object's class name, and adding one is a
new value shape for the sake of a message.

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
  per-callable growth. The parameter tags and the arity stay on every callable value. The entry
  check spends one scan of the argument's ancestry per class-declared parameter per call, and
  allocates nothing.

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
- **Name-keyed access for every named-class property.** It closes the same hole everywhere, but it
  spends latency in every program, most of which never write an anonymous function.
- **A class channel in the parameter tags.** A per-callable list of descriptors would spend an
  allocation at every anonymous function literal to answer a question the body's first block asks
  for free.
