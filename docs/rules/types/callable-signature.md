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
per-argument check, and nothing existing changes meaning. Narrowing is opt-in. Where a call reaches a
callable whose type names its parameters, the arguments are proven at compile time and the
per-argument check is **not run**; the metadata stays on every callable class, because bare `callable`
still needs it and an anonymous function does not know where it is written which kind of site will call it.

**The reason is safety, not convenience.** A compiled callable reads each argument at its own declared
representation, so a mismatched argument is reinterpreted: an `int` read as a string pointer is an
arbitrary dereference, not a fault. A bare `callable` guards that with a runtime check per argument,
per call, of each parameter's whole declared type: the check a write by name runs against a property
of that type (`rule:types/erased-member-access`). It reads the callable class's `invoke` method row,
which carries each parameter's check as any method row does, so a `?C`, an `array<int>`, an enum and
a `callable(int): string` parameter are each tested as `is` tests them, and the one `int` to `float`
widening of a parameter declared `float` is kept. A miss throws a `LogicError` that names the
position and the value given. A written signature discharges the same guard at compile time, as an error where
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

The entry check is for a proven call, which does not run the row check, and for the paths that reach
the body with no row check at all. A `?C` parameter is not tested at the entry: it erases to a tagged
value before any class survives. A bare `callable` call tests it from the row, and a written signature
proves it where the call is written. The entry's error does not name the class that arrived, because
no compiled instruction reads an object's class name, and adding one is a new value shape for the sake
of a message. The row check's error names it, because it runs in the runtime, which reads the
argument's descriptor.

**A call with no parameter list refuses a qualified argument where it is written** (`E0895`). The
run-time check tests each argument's representation, and neither `tainted` nor `secret` has one, so a
callee's plain `string` parameter would take the value as trusted text. An argument that carries
`tainted`, `secret` or `mixed` anywhere in it does not compile through a bare `callable`, or through
the method call on a `mixed` receiver that `rule:types/erased-member-access` defers. `mixed` counts,
because text out of it is `tainted` (`rule:security/taint-propagation`). The form that compiles writes
the parameter types, such as `callable(tainted string): void` or `callable(mixed): void`, and the
signature then checks each argument. A `...` argument into a written signature is refused the same
way unless every parameter takes its element type, because the call site cannot pair an element with
its parameter. It costs nothing at run time.

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
  per-callable growth. The parameter tags and the arity stay on every callable value, and the
  `invoke` row's two small vectors of checks on every callable class, once per process. A bare
  `callable` call spends one class walk per class-typed argument and one element walk per
  typed-array argument. The entry check spends one scan of the argument's ancestry per
  class-declared parameter per call. Neither allocates.

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
