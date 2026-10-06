```nvs
function Core\Program::constructors<T, C>(): array<{class: string, make: C}>;   // C is callable(...): T
```

It expands, while compiling, to an array literal of one row per class `rule:programs/implementing`'s
`implementing<T>()` would list, **in the same order**, so the two can be zipped. `class` is the
fully-qualified name. `make` is the ordinary anonymous function `fn(<C's parameters>): T => new Class(<the same
arguments, in order>)`, checked exactly as that anonymous function would be if it were written at the call site —
argument types, defaults and the visibility `new` faces there. No new calling path and no reflection is
involved: the rows are what a program could have written by hand, had it known the list.

`C` must be a `callable(...)` type whose return type is `T`, or the call is refused. A class whose
constructor that anonymous function does not fit is refused on the call, naming the class, with the ordinary error
attached as a note. The enumeration's no-argument-constructor demand does not apply here: this member
is how a class whose constructor takes dependencies is enumerated.

Nothing is built until `make` is called. Each call allocates one callable per listed class, charged to
the request and freed with the array, and calling it opts the program into the same scan the other
enumeration members do.
