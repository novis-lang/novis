```nvs
function Core\Program::constructors<T, C>(): array<{class: string, make: C}>;   // C is callable(...): T
```

It expands, while compiling, to an array literal of one row per class `rule:programs/implementing`'s
`implementing<T>()` would list, **in the same order**, so the two can be zipped. `class` is the
fully-qualified name. `make` is the ordinary anonymous function `fn(<C's parameters>): T => new Class(<the same
arguments, in order>)`, checked exactly as that anonymous function would be if it were written at the call site —
argument types, defaults and the visibility `new` faces there. No new calling path and no reflection is
involved: the rows are what a program could have written by hand, had it known the list.

`C` must be a `callable(...)` type whose return type is `T`, or the call is refused (`E0838`). A class
whose constructor that anonymous function does not fit — a wrong argument type, a missing required
parameter, a constructor not visible at the call site — is refused on the call (`E0839`), naming the
class, with the ordinary error attached as a note. The enumeration's no-argument-constructor demand
(`E0744`) does not apply here: this member is how a class whose constructor takes dependencies is
enumerated. Without it, a dependency arrives through a method called after construction, so an object
is reachable half-built and every class owes a setter its type cannot require.

Nothing is built until `make` is called, and calling `constructors` opts the program into the same
scan the other enumeration members do.

- **Memory:** each call allocates one callable per listed class, charged to the request and freed with
  the array, and no listed object until its `make` is called.

**Declined:**
- **Class names to pass to `Core\Reflect::construct`.** It moves the constructor check from the call
  site to run time, and the arguments lose their static types.
- **Keeping the no-argument-constructor limit and documenting the second step.** It leaves an object
  reachable half-built between construction and the setter call.
